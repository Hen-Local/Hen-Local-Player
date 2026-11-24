use super::{
    store::{self, Audio},
    timeline::{self, Timeline},
    vad, Message, ModelUse, Request, SharedEngines,
};
use anyhow::{bail, Context as _, Result};
use hlp_core::{
    asr_text::{self, ParsedAsr},
    cues::AlignedPhrase,
    events::*,
    lang,
    settings::Settings,
    types::*,
};
use hlp_media::Tools;
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::sync::CancellationToken;

pub(crate) struct Context {
    pub(super) counts: Arc<super::Counts>,
    pub automatic_transcription: bool,
    pub root: PathBuf,
    pub project: store::Project,
    pub settings: Settings,
    pub caps: Caps,
    pub events: Arc<dyn PipelineEvents>,
    pub shared: SharedEngines,
    pub tools: Tools,
    pub id: String,
    pub opened_at: u64,
    pub playhead: Ms,
    pub playing: bool,
}
impl Context {
    fn dir(&self) -> PathBuf {
        self.root.join("media").join(&self.project.fingerprint)
    }
    fn prepared_path(&self) -> PathBuf {
        let extension = if self.project.probe.video.is_none() {
            "m4a"
        } else {
            "mp4"
        };
        self.dir().join(format!(
            "playable-{}.{extension}",
            self.project.audio_track_order
        ))
    }
    fn error(&self, code: &str, message: String, recoverable: bool) {
        self.events.error(ErrorEvent {
            session_id: Some(self.id.clone()),
            code: code.into(),
            message,
            recoverable,
        })
    }
    fn persist(&mut self, t: &Timeline) -> Result<()> {
        self.project.updated_at = store::now();
        self.project.language = t.language.clone();
        store::write_json(&self.dir().join("project.json"), &self.project)?;
        t.persist()?;
        let s = t.status("");
        let fraction = if s.utt_total > 0 {
            (s.utt_done + s.utt_no_speech) as f32 / s.utt_total as f32
        } else if t.vad_complete {
            1.
        } else {
            0.
        };
        store::record_recent(&self.root, &self.project, fraction, self.opened_at)
    }
}

enum Work {
    Region(Utterance),
    Position(Ms),
    Extracted(Result<u64>),
    Prep(f32),
    Playable(Result<PathBuf>),
    Asr {
        epoch: u64,
        id: String,
        forced: Option<LangCode>,
        result: Result<ParsedAsr>,
        audio: u64,
        elapsed: u64,
    },
    Translated {
        epoch: u64,
        id: String,
        source_hash: String,
        context_hash: String,
        result: Result<(String, Vec<AlignedPhrase>)>,
        elapsed: u64,
    },
}
struct Running {
    tx: mpsc::UnboundedSender<Work>,
    tasks: JoinSet<()>,
    cancel: CancellationToken,
    generate: bool,
    regenerating: bool,
    extracting: bool,
    asr: bool,
    mt: bool,
    prep: bool,
    asr_epoch: u64,
    tr_epoch: u64,
    setup: bool,
    engine_failed: bool,
    prep_failed: bool,
    model_use: Option<ModelUse>,
    seen: HashSet<String>,
    url: Option<String>,
}
impl Running {
    fn prepare(&mut self, ctx: &Context) {
        let plan = ctx.project.playback.plan.clone();
        let output = ctx.prepared_path();
        let (tx, tools, path, info, caps, order, cancel) = (
            self.tx.clone(),
            ctx.tools.clone(),
            ctx.project.source_path.clone(),
            ctx.project.probe.clone(),
            ctx.caps.clone(),
            ctx.project.audio_track_order,
            self.cancel.clone(),
        );
        self.prep = true;
        self.tasks.spawn_blocking(move || {
            let r = hlp_media::prepare_playback(
                &tools,
                &path,
                &info,
                &caps,
                order,
                &plan,
                &output,
                |p| {
                    let _ = tx.send(Work::Prep(p));
                },
                cancel,
            );
            let _ = tx.send(Work::Playable(r));
        });
    }
    fn extract(&mut self, ctx: &Context, t: &Timeline) -> Result<()> {
        if !self.generate || self.extracting || t.vad_complete {
            return Ok(());
        }
        let pcm = t.track.join("audio.pcm");
        let focus = ctx.settings.dialogue_channel_focus;
        let Some(track) = ctx
            .project
            .probe
            .audio
            .iter()
            .find(|a| a.order == ctx.project.audio_track_order)
            .cloned()
        else {
            self.setup = true;
            return Ok(());
        };
        let vad = hlp_models::vad_path();
        if !vad.is_file() || (!t.audio_complete && !ctx.tools.ffmpeg.is_file()) {
            self.setup = true;
            return Ok(());
        }
        let reuse = t.audio_complete;
        if !reuse {
            store::write_json(
                &t.track.join("audio.json"),
                &Audio {
                    sample_rate: 16000,
                    total_samples: 0,
                    complete: false,
                    dialogue_focus: focus,
                },
            )?
        }
        let (tools, path, tx, cancel) = (
            ctx.tools.clone(),
            ctx.project.source_path.clone(),
            self.tx.clone(),
            self.cancel.clone(),
        );
        self.extracting = true;
        self.tasks.spawn_blocking(move || {
            let result = std::thread::scope(|scope| -> Result<u64> {
                let (frames, incoming) = crossbeam_channel::bounded::<(Vec<f32>, Ms)>(64);
                let vad_tx = tx.clone();
                let vad_cancel = cancel.clone();
                let detection = scope.spawn(move || -> Result<()> {
                    let mut detector = vad::Detector::new(&vad)?;
                    for (samples, position) in incoming {
                        anyhow::ensure!(!vad_cancel.is_cancelled(), "Session cancelled");
                        detector.feed(&samples, position, |u| {
                            let _ = vad_tx.send(Work::Region(u));
                        });
                    }
                    detector.finish(|u| {
                        let _ = vad_tx.send(Work::Region(u));
                    });
                    Ok(())
                });
                let mut last = 0;
                let mut feed = |samples: &[f32], ms: Ms| -> Result<()> {
                    anyhow::ensure!(!cancel.is_cancelled(), "Session cancelled");
                    frames
                        .send((samples.to_vec(), ms))
                        .context("VAD stopped consuming audio")?;
                    if ms >= last + 250 {
                        let _ = tx.send(Work::Position(ms));
                        last = ms;
                    }
                    Ok(())
                };
                let extraction = if reuse {
                    read_frames(&pcm, &mut feed)
                } else {
                    hlp_media::extract_audio(
                        &tools,
                        &path,
                        &track,
                        focus,
                        &pcm,
                        &mut feed,
                        cancel.clone(),
                    )
                };
                drop(frames);
                detection
                    .join()
                    .map_err(|_| anyhow::anyhow!("VAD thread stopped"))??;
                extraction
            });
            let _ = tx.send(Work::Extracted(result));
        });
        Ok(())
    }
    fn schedule(&mut self, ctx: &Context, t: &mut Timeline) {
        if !self.generate || self.setup || self.engine_failed {
            return;
        }
        if !self.asr {
            if let Some(u) = t.pick(false, ctx.playhead, &ctx.settings) {
                let model = hlp_models::asr_model_dir(&t.model);
                if !model.is_dir() {
                    self.setup = true;
                    return;
                }
                let u = u.clone();
                let epoch = self.asr_epoch;
                let forced = if t.language.locked && !cfg!(windows) {
                    t.language.detected.clone()
                } else {
                    None
                };
                let retry_language = t.language.detected.clone().or(u.language.clone());
                let (client, tx, pcm) = (
                    ctx.shared.asr.clone(),
                    self.tx.clone(),
                    t.track.join("audio.pcm"),
                );
                let counts = ctx.counts.clone();
                self.model_use
                    .get_or_insert_with(|| ctx.shared.use_models());
                self.asr = true;
                self.tasks.spawn(async move {
                    let start = Instant::now();
                    let speech = timeline::speech_ms(&u);
                    let mut used = forced;
                    let result = async {
                        let mut client = client.lock().await;
                        let budget =
                            (48 + 20 * (u.end_ms - u.start_ms).div_ceil(1000)).min(1024) as u32;
                        for attempt in 0..2 {
                            let lang = used
                                .as_ref()
                                .and_then(|c| lang::lookup(&c.0))
                                .and_then(|l| l.qwen3_asr_name);
                            counts
                                .asr
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let out = client
                                .transcribe(
                                    &pcm,
                                    u.start_ms * 16,
                                    u.end_ms * 16,
                                    lang,
                                    if attempt == 0 { budget } else { budget / 2 },
                                )
                                .await?;
                            let parsed = asr_text::parse(&out.raw, speech as f64 / 1000.);
                            if !matches!(parsed, ParsedAsr::Failed(_)) {
                                return Ok(parsed);
                            }
                            if !cfg!(windows) && used.is_none() {
                                used = retry_language.clone()
                            }
                        }
                        Ok(ParsedAsr::NoSpeech)
                    }
                    .await;
                    let _ = tx.send(Work::Asr {
                        epoch,
                        id: u.id,
                        forced: used,
                        result,
                        audio: u.end_ms - u.start_ms,
                        elapsed: start.elapsed().as_millis() as u64,
                    });
                });
            }
        }
        if !self.mt {
            if let Some(u) = t.pick(true, ctx.playhead, &ctx.settings) {
                if cfg!(target_os = "macos")
                    && ctx.playing
                    && t.defer_translation(u, ctx.playhead, ctx.settings.lookahead_seconds)
                {
                    return;
                }
                if !hlp_models::gguf_path().is_file() || !hlp_models::llama_server_path().is_file()
                {
                    self.setup = true;
                    return;
                }
                let source = t.source_for(u).map(str::to_string);
                let text = u.text.clone().unwrap_or_default();
                let bg = t.background(u);
                let context_hash = t.context_hash(u, bg.as_deref());
                let (id, target, epoch, tx, mt) = (
                    u.id.clone(),
                    t.target.clone(),
                    self.tr_epoch,
                    self.tx.clone(),
                    ctx.shared.translator.clone(),
                );
                self.mt = true;
                self.model_use
                    .get_or_insert_with(|| ctx.shared.use_models());
                ctx.counts
                    .translation
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.tasks.spawn(async move {
                    let start = Instant::now();
                    let result = mt
                        .lock()
                        .await
                        .translate_subtitle(source.as_deref(), &target, &text, bg.as_deref())
                        .await;
                    let _ = tx.send(Work::Translated {
                        epoch,
                        id,
                        source_hash: store::hash(&text),
                        context_hash,
                        result,
                        elapsed: start.elapsed().as_millis() as u64,
                    });
                });
            }
        }
    }
}
fn read_frames(path: &Path, mut emit: impl FnMut(&[f32], Ms) -> Result<()>) -> Result<u64> {
    let f = File::open(path)?;
    let len = f.metadata()?.len() / 2;
    let mut reader = BufReader::new(f);
    let mut buf = [0; 1024];
    let mut samples = [0.; 512];
    let mut offset = 0;
    while offset < len {
        let n = (len - offset).min(512) as usize;
        reader.read_exact(&mut buf[..2 * n])?;
        for (sample, chunk) in samples.iter_mut().zip(buf[..2 * n].chunks_exact(2)) {
            *sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.
        }
        offset += n as u64;
        emit(&samples[..n], offset / 16)?;
    }
    Ok(len)
}
pub(super) async fn serve(ctx: &mut Context, mut requests: mpsc::Receiver<Request>) -> Result<()> {
    std::fs::create_dir_all(ctx.dir())?;
    store::evict(
        &ctx.root,
        &ctx.project.fingerprint,
        ctx.settings.cache_limit_gb as u64 * 1024u64.pow(3),
    )?;
    loop {
        let track = ctx
            .dir()
            .join(format!("track-{}", ctx.project.audio_track_order));
        let mut timeline = Timeline::load(track, &ctx.settings, &ctx.id)?;
        if ctx.project.probe.video.is_some() && ctx.project.probe.audio.is_empty() {
            // Silent recordings can play immediately and have no recognition work.
            timeline.vad_complete = true;
            timeline.audio_complete = true;
            timeline.extracted = ctx.project.duration_ms;
        }
        // A different track or an earlier decoder fallback may already have produced
        // a complete playable for this exact WebView capability set.
        if let Some(prepared) =
            store::prepared_playback(&timeline.track, &ctx.prepared_path(), &ctx.caps)
        {
            ctx.project.playback = prepared;
        } else if timeline.track.join("playback.json").is_file() {
            ctx.project.playback.playable_path = None;
        }
        let (tx, work) = mpsc::unbounded_channel();
        let mut run = Running {
            tx,
            tasks: JoinSet::new(),
            cancel: CancellationToken::new(),
            generate: ctx.automatic_transcription
                || timeline.track.join("generate.requested").is_file()
                || !timeline.rows.is_empty(),
            regenerating: false,
            extracting: false,
            asr: false,
            mt: false,
            prep: false,
            asr_epoch: 0,
            tr_epoch: 0,
            setup: ctx.project.probe.audio.is_empty() && ctx.project.probe.video.is_none(),
            engine_failed: false,
            prep_failed: false,
            model_use: None,
            seen: HashSet::new(),
            url: None,
        };
        let plan = &ctx.project.playback.plan;
        let playable = if *plan == PlaybackPlan::Direct {
            Some(ctx.project.source_path.clone())
        } else {
            ctx.project
                .playback
                .playable_path
                .clone()
                .filter(|p| p.is_file())
        };
        run.url = playable
            .map(|p| ctx.shared.server.register(&p))
            .transpose()?;
        let cached = timeline.cues();
        timeline.dirty = false;
        ctx.events.media_opened(MediaOpened {
            session_id: ctx.id.clone(),
            title: ctx.project.title.clone(),
            duration_ms: ctx.project.duration_ms,
            media_url: run.url.clone(),
            plan: plan.clone(),
            audio_tracks: ctx
                .project
                .probe
                .audio
                .iter()
                .cloned()
                .map(|mut a| {
                    a.default = a.order == ctx.project.audio_track_order;
                    a
                })
                .collect(),
            language: timeline.language.clone(),
            target: timeline.target.clone(),
            cues: cached.clone(),
        });
        if run.url.is_none() {
            run.prepare(ctx)
        }
        run.extract(ctx, &timeline)?;
        let result = drive(ctx, &mut timeline, &mut run, &mut requests, work, cached).await;
        run.cancel.cancel();
        run.tasks.abort_all();
        while run.tasks.join_next().await.is_some() {}
        if let Some(url) = &run.url {
            ctx.shared.server.unregister(url)
        }
        ctx.persist(&timeline)?;
        match result? {
            Next::Stop => break,
            Next::Restart {
                order,
                model,
                focus,
            } => {
                ctx.project.audio_track_order = order;
                ctx.project.playback = store::Playback {
                    plan: hlp_media::decide(&ctx.project.probe, &ctx.caps, order),
                    playable_path: None,
                };
                if model != ctx.settings.asr_model {
                    let mut asr = ctx.shared.asr.lock().await;
                    asr.shutdown().await;
                    *asr = super::AsrClient::new(hlp_models::asr_model_dir(&model));
                    ctx.settings.asr_model = model;
                }
                ctx.settings.dialogue_channel_focus = focus;
            }
        }
    }
    Ok(())
}
enum Next {
    Stop,
    Restart {
        order: usize,
        model: String,
        focus: bool,
    },
}
fn diff_cues(ctx: &Context, t: &mut Timeline, previous: &mut BTreeMap<String, Cue>) {
    let next: BTreeMap<_, _> = t.cues().into_iter().map(|c| (c.id.clone(), c)).collect();
    let upserts: Vec<_> = next
        .iter()
        .filter(|(id, c)| previous.get(*id) != Some(*c))
        .map(|(_, c)| c.clone())
        .collect();
    let removed_ids: Vec<_> = previous
        .keys()
        .filter(|id| !next.contains_key(*id))
        .cloned()
        .collect();
    if !upserts.is_empty() || !removed_ids.is_empty() {
        ctx.events.cues(CueBatch {
            session_id: ctx.id.clone(),
            upserts,
            removed_ids,
        })
    }
    *previous = next;
}
async fn drive(
    ctx: &mut Context,
    t: &mut Timeline,
    run: &mut Running,
    requests: &mut mpsc::Receiver<Request>,
    mut work: mpsc::UnboundedReceiver<Work>,
    cached: Vec<Cue>,
) -> Result<Next> {
    let mut previous = cached.into_iter().map(|c| (c.id.clone(), c)).collect();
    let mut last_cues = Instant::now();
    let mut last_save = Instant::now();
    let mut last_status = Instant::now() - Duration::from_secs(1);
    let mut stage_before = String::new();
    let mut pending_save = true;
    let mut ticker = tokio::time::interval(Duration::from_millis(100));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        // Once complete, idle ticks and playhead reports cannot add work. Transcript,
        // audio, language and translation changes all mark the timeline dirty.
        let completed = stage_before == "complete" && !t.dirty;
        if !completed {
            run.schedule(ctx, t);
        }
        let stage =
            if run.prep_failed || run.engine_failed {
                "error"
            } else if run.setup {
                "needs-setup"
            } else if !run.generate {
                "idle"
            } else if t.vad_complete && !run.asr && !run.mt && !run.prep {
                if completed {
                    "complete"
                } else if t
                    .rows
                    .values()
                    .any(|u| u.status == UttStatus::Pending || t.tr_needed(u))
                {
                    "paused"
                } else if t.rows.values().any(|u| {
                    matches!(u.status, UttStatus::Failed { .. }) || t.translation_failed(u)
                }) {
                    "error"
                } else {
                    "complete"
                }
            } else {
                "processing"
            };
        let changed = stage_before != stage;
        if stage != "processing" && !run.asr && !run.mt {
            run.model_use = None;
        }
        if matches!(stage, "complete" | "error") {
            run.regenerating = false;
        }
        if t.dirty && (last_cues.elapsed() >= Duration::from_millis(250) || stage == "complete") {
            diff_cues(ctx, t, &mut previous);
            t.dirty = false;
            pending_save = true;
            last_cues = Instant::now();
        }
        if t.language_dirty {
            ctx.events.language(t.language.clone());
            t.language_dirty = false
        }
        if pending_save
            && (last_save.elapsed() >= Duration::from_secs(2) || (changed && stage == "complete"))
        {
            ctx.persist(t)?;
            pending_save = false;
            last_save = Instant::now();
        }
        if changed || last_status.elapsed() >= Duration::from_millis(500) && stage == "processing" {
            ctx.events.status(t.status(stage));
            stage_before = stage.into();
            last_status = Instant::now()
        }
        tokio::select! {biased;
            request=requests.recv()=>{
                let Some(request)=request else{return Ok(Next::Stop)};
                let result=handle_command(ctx,t,run,request.action);
                match result{Ok(next)=>{let _=request.reply.send(Ok(()));if let Some(next)=next{return Ok(next)}},Err(e)=>{let _=request.reply.send(Err(e));}}
            }
            Some(event)=work.recv()=>apply_work(ctx,t,run,event)?,
            Some(result)=run.tasks.join_next(),if !run.tasks.is_empty()=>{result.context("Pipeline task stopped")?;}
            _=ticker.tick(),if stage == "processing" || pending_save || t.dirty=>{}
        }
    }
}
fn apply_work(ctx: &mut Context, t: &mut Timeline, run: &mut Running, event: Work) -> Result<()> {
    match event {
        Work::Position(ms) => t.extracted = t.extracted.max(ms),
        Work::Region(u) => {
            run.seen.insert(u.id.clone());
            if !t
                .rows
                .get(&u.id)
                .is_some_and(|old| old.regions == u.regions)
            {
                let id = u.id.clone();
                t.rows.insert(id.clone(), u);
                t.refresh_keys_from(&id);
                t.dirty = true
            }
        }
        Work::Extracted(result) => {
            run.extracting = false;
            match result {
                Ok(n) => {
                    t.extracted = n / 16;
                    t.audio_complete = true;
                    t.vad_complete = true;
                    t.rows.retain(|id, _| run.seen.contains(id));
                    t.rebuild_keys();
                    t.dirty = true;
                    store::write_json(
                        &t.track.join("audio.json"),
                        &Audio {
                            sample_rate: 16000,
                            total_samples: n,
                            complete: true,
                            dialogue_focus: ctx.settings.dialogue_channel_focus,
                        },
                    )?;
                }
                Err(e) => {
                    run.engine_failed = true;
                    ctx.error("Extraction", format!("{e:#}"), true)
                }
            }
        }
        Work::Prep(progress) => ctx.events.playback_prep(PlaybackPrep {
            session_id: ctx.id.clone(),
            plan: ctx.project.playback.plan.clone(),
            progress,
            media_url: None,
            error: None,
        }),
        Work::Playable(result) => {
            run.prep = false;
            let (url, error) = match result {
                Ok(p) => {
                    ctx.project.playback.playable_path = Some(p.clone());
                    store::write_json(
                        &t.track.join("playback.json"),
                        &store::Prepared {
                            caps: ctx.caps.clone(),
                            plan: ctx.project.playback.plan.clone(),
                        },
                    )?;
                    (Some(ctx.shared.server.register(&p)?), None)
                }
                Err(e) => {
                    run.prep_failed = true;
                    (None, Some(format!("{e:#}")))
                }
            };
            run.url = url.clone();
            ctx.events.playback_prep(PlaybackPrep {
                session_id: ctx.id.clone(),
                plan: ctx.project.playback.plan.clone(),
                progress: 1.,
                media_url: url,
                error,
            });
            t.dirty = true;
        }
        Work::Asr {
            epoch,
            id,
            forced,
            result,
            audio,
            elapsed,
        } => {
            run.asr = false;
            if epoch == run.asr_epoch {
                match result {
                    Ok(parsed) => {
                        t.audio_ms += audio;
                        t.asr_ms += elapsed;
                        t.apply(&id, parsed, forced)
                    }
                    Err(e) => {
                        if let Some(u) = t.rows.get_mut(&id) {
                            u.status = UttStatus::Failed {
                                attempts: 1,
                                error: format!("{e:#}"),
                            }
                        }
                        run.engine_failed = true;
                        t.dirty = true;
                        ctx.error("AsrUnavailable", format!("{e:#}"), true)
                    }
                }
            }
        }
        Work::Translated {
            epoch,
            id,
            source_hash,
            context_hash,
            result,
            elapsed,
        } => {
            run.mt = false;
            let current = t.rows.get(&id).is_some_and(|u| {
                u.status == UttStatus::Done
                    && u.text
                        .as_deref()
                        .is_some_and(|text| store::hash(text) == source_hash)
                    && t.context_hash(u, t.background(u).as_deref()) == context_hash
            });
            if epoch == run.tr_epoch && current {
                let source_language = t
                    .rows
                    .get(&id)
                    .and_then(|u| t.source_for(u))
                    .map(str::to_owned);
                let failed = result.is_err();
                let (text, aligned_phrases) = match result {
                    Ok(translation) => translation,
                    Err(e) => {
                        ctx.error("Translation", format!("{e:#}"), true);
                        t.translations
                            .items
                            .get(&id)
                            .filter(|v| {
                                v.source_hash == source_hash
                                    && v.source_language
                                        .as_ref()
                                        .is_none_or(|lang| Some(lang) == source_language.as_ref())
                            })
                            .map(|v| (v.text.clone(), v.aligned_phrases.clone()))
                            .unwrap_or_default()
                    }
                };
                t.tr_count += 1;
                t.tr_ms += elapsed;
                t.translations.items.insert(
                    id,
                    store::Translation {
                        source_hash,
                        context_hash: Some(context_hash),
                        source_language,
                        text,
                        aligned_phrases,
                        failed,
                    },
                );
                t.dirty = true;
            }
        }
    }
    Ok(())
}
fn resume_transcription(ctx: &mut Context, t: &mut Timeline, run: &mut Running) -> Result<()> {
    for u in t.rows.values_mut() {
        if matches!(u.status, UttStatus::Failed { .. }) {
            u.status = UttStatus::Pending;
        }
    }
    t.translations.items.retain(|_, v| !v.failed);
    run.engine_failed = false;
    run.setup = false;
    t.dirty = true;
    ctx.tools = Tools::resolve();
    run.extract(ctx, t)
}
fn handle_command(
    ctx: &mut Context,
    t: &mut Timeline,
    run: &mut Running,
    action: Message,
) -> Result<Option<Next>> {
    let command = match action {
        Message::SourceUrl(url) => {
            ctx.project.source_url = Some(url);
            ctx.persist(t)?;
            return Ok(None);
        }
        Message::Command(c) => c,
    };
    match command {
        SessionCommand::Shutdown => return Ok(Some(Next::Stop)),
        SessionCommand::Playhead { ms, playing } => {
            ctx.playhead = ms;
            ctx.playing = playing;
        }
        SessionCommand::SetTarget(code) => {
            let mut s = ctx.settings.clone();
            s.target_language = code.clone();
            timeline::validate_settings(&s)?;
            run.tr_epoch += 1;
            t.set_target(&code, s.translation_context)?;
            ctx.settings = s
        }
        SessionCommand::SetSource(mode) => {
            let mut s = ctx.settings.clone();
            s.source_language = mode.clone();
            timeline::validate_settings(&s)?;
            run.tr_epoch += 1;
            run.asr_epoch += 1;
            t.set_source(&mode);
            ctx.settings = s
        }
        SessionCommand::SelectAudioTrack(order) => {
            anyhow::ensure!(
                ctx.project.probe.audio.iter().any(|a| a.order == order),
                "Unknown audio track"
            );
            return Ok(Some(Next::Restart {
                order,
                model: ctx.settings.asr_model.clone(),
                focus: ctx.settings.dialogue_channel_focus,
            }));
        }
        SessionCommand::UpdateSettings(mut s) => {
            timeline::validate_settings(&s)?;
            if t.target != s.target_language
                || ctx.settings.translation_context != s.translation_context
            {
                run.tr_epoch += 1;
                t.set_target(&s.target_language, s.translation_context)?
            }
            if ctx.settings.source_language != s.source_language {
                run.tr_epoch += 1;
                run.asr_epoch += 1;
                t.set_source(&s.source_language)
            }
            if s.asr_model != ctx.settings.asr_model
                || s.dialogue_channel_focus != ctx.settings.dialogue_channel_focus
            {
                let next = Next::Restart {
                    order: ctx.project.audio_track_order,
                    model: s.asr_model.clone(),
                    focus: s.dialogue_channel_focus,
                };
                s.asr_model = ctx.settings.asr_model.clone();
                s.dialogue_channel_focus = ctx.settings.dialogue_channel_focus;
                ctx.settings = s;
                return Ok(Some(next));
            }
            ctx.settings = s;
        }
        SessionCommand::Export {
            format,
            variant,
            path,
        } => store::atomic_write(
            &path,
            hlp_core::subs::render(&t.cues(), &format, &variant).as_bytes(),
        )?,
        SessionCommand::StartTranscription {
            components_ready,
            regenerate,
            audio_track_order,
        } => {
            anyhow::ensure!(
                audio_track_order.is_none_or(|order| order == ctx.project.audio_track_order),
                "The selected audio track changed. Try again."
            );
            if !components_ready {
                run.setup = true;
                return Ok(None);
            }
            anyhow::ensure!(
                !ctx.project.probe.audio.is_empty(),
                "This file has no audio track to transcribe."
            );
            // Persist only an accepted request. Reopening resumes a partial
            // transcription; visiting the subtitles panel alone never starts one.
            store::atomic_write(&t.track.join("generate.requested"), b"1")?;
            if regenerate && !run.regenerating {
                t.regenerate(&ctx.settings.source_language)?;
                run.asr_epoch += 1;
                run.tr_epoch += 1;
                run.regenerating = true;
            }
            run.generate = true;
            resume_transcription(ctx, t, run)?;
        }
        SessionCommand::RetryFailed => {
            // A development install can open a file before ffprobe is available.
            // Refresh track metadata and the playback plan once setup supplies it.
            if ctx.project.probe.audio.is_empty() && ctx.project.probe.video.is_none() {
                ctx.tools = Tools::resolve();
                if ctx.tools.ffprobe.is_file() {
                    ctx.project.probe = hlp_media::probe(&ctx.tools, &ctx.project.source_path)?;
                    ctx.project.duration_ms = ctx.project.probe.duration_ms;
                    return Ok(Some(Next::Restart {
                        order: ctx.project.audio_track_order,
                        model: ctx.settings.asr_model.clone(),
                        focus: ctx.settings.dialogue_channel_focus,
                    }));
                }
            }
            if run.generate {
                resume_transcription(ctx, t, run)?;
            }
        }
        SessionCommand::PlaybackError(code) => {
            if !matches!(code, 3 | 4) || run.prep {
                return Ok(None);
            }
            let next = match ctx.project.playback.plan {
                PlaybackPlan::Direct => PlaybackPlan::Remux,
                PlaybackPlan::Remux => PlaybackPlan::Transcode,
                PlaybackPlan::Transcode => bail!("WebView could not decode the transcoded video"),
            };
            if let Some(url) = run.url.take() {
                ctx.shared.server.unregister(&url)
            }
            ctx.project.playback = store::Playback {
                plan: next,
                playable_path: None,
            };
            run.prep_failed = false;
            run.prepare(ctx);
        }
    }
    Ok(None)
}
