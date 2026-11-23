#[derive(clap::Args)]
pub struct Args {
    pub input: String,
    #[arg(long, default_value = "auto")]
    pub source: String,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long, value_delimiter = ',', default_value = "srt,vtt,ass",value_parser=["srt","vtt","ass"])]
    pub formats: Vec<String>,
    #[arg(long)]
    pub out_dir: Option<std::path::PathBuf>,
    #[arg(long)]
    pub no_cache: bool,
    #[arg(long)]
    pub json_progress: bool,
}
use anyhow::{anyhow, Context, Result};
use hlp_core::{events::*, settings::Settings, types::*};
use hlp_pipeline::{store, Session, SharedEngines};
use serde::Serialize;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[derive(Default, Clone)]
struct Snapshot {
    language: Option<LanguageEvent>,
    status: Option<PipelineStatus>,
    error: Option<(i32, String)>,
}
struct Printer {
    json: bool,
    state: Mutex<Snapshot>,
    finished: watch::Sender<bool>,
}
impl Printer {
    fn emit(&self, event: &str, payload: &impl Serialize) {
        if self.json {
            println!("{}", json!({"event":event,"payload":payload}))
        }
    }
}
impl PipelineEvents for Printer {
    fn media_opened(&self, e: MediaOpened) {
        self.emit("media://opened", &e);
        self.state.lock().unwrap().language = Some(e.language);
    }
    fn language(&self, e: LanguageEvent) {
        self.emit("pipeline://language", &e);
        self.state.lock().unwrap().language = Some(e);
    }
    fn cues(&self, e: CueBatch) {
        self.emit("pipeline://cues", &e);
    }
    fn playback_prep(&self, e: PlaybackPrep) {
        self.emit("media://prep", &e);
        if let Some(s) = e.error {
            self.state.lock().unwrap().error = Some((3, s));
        }
    }
    fn status(&self, e: PipelineStatus) {
        self.emit("pipeline://status", &e);
        let end = matches!(e.stage.as_str(), "complete" | "error" | "needs-setup");
        if !self.json {
            eprint!(
                "\r{} · {}/{} transcribed · {} translated     ",
                e.stage, e.utt_done, e.utt_total, e.translated_count
            );
        }
        self.state.lock().unwrap().status = Some(e);
        if end {
            self.finished.send_replace(true);
        }
    }
    fn error(&self, e: ErrorEvent) {
        self.emit("app://error", &e);
        let code = if matches!(e.code.as_str(), "AsrUnavailable" | "Translation") {
            4
        } else {
            3
        };
        self.state.lock().unwrap().error = Some((code, e.message));
        if !e.recoverable {
            self.finished.send_replace(true);
        }
    }
}
async fn input_path(input: &str, dir: &Path, printer: &Printer) -> Result<PathBuf> {
    if !input.starts_with("https://") && !input.starts_with("http://") {
        return Ok(PathBuf::from(input));
    }
    let tools = hlp_media::Tools::resolve();
    let ids: Vec<String> = [("yt-dlp", tools.yt_dlp), ("deno", tools.deno)]
        .into_iter()
        .filter(|(_, p)| !p.is_file())
        .map(|(id, _)| id.to_owned())
        .collect();
    let cancel = CancellationToken::new();
    if !ids.is_empty() {
        hlp_models::download(
            &ids,
            |e| printer.emit("setup://progress", &e),
            cancel.clone(),
        )
        .await
        .context(crate::ExitCode(2))?;
    }
    let downloader = hlp_media::Downloader::new(hlp_media::Tools::resolve());
    let info = downloader
        .probe(input, cancel.clone())
        .await
        .context(crate::ExitCode(3))?;
    downloader
        .download(
            input,
            &info,
            dir,
            |e| printer.emit("download://progress", &e),
            cancel,
        )
        .await
        .context(crate::ExitCode(3))
}
pub async fn run(args: Args) -> Result<()> {
    let start = Instant::now();
    let temporary = if args.no_cache {
        Some(tempfile::tempdir()?)
    } else {
        None
    };
    let root = temporary
        .as_ref()
        .map(|d| d.path().to_owned())
        .unwrap_or_else(store::data_root);
    let mut settings: Settings =
        store::read(&store::data_root().join("settings.json")).unwrap_or_default();
    settings.process_whole_file = true;
    settings.source_language = args.source;
    if let Some(target) = args.to {
        settings.target_language = target
    }
    let target = settings.target_language.clone();
    let (finished, mut done) = watch::channel(false);
    let printer = Arc::new(Printer {
        json: args.json_progress,
        state: Mutex::default(),
        finished,
    });
    let path = input_path(&args.input, &settings.download_dir, &printer).await?;
    let shared = SharedEngines::new(&settings).await?;
    let caps = hlp_media::default_caps(if cfg!(windows) {
        "webview2"
    } else {
        "wkwebview"
    });
    let handle = Session::open_in(
        path.clone(),
        settings.clone(),
        caps,
        printer.clone(),
        shared.clone(),
        root,
    )
    .await
    .context(crate::ExitCode(3))?;
    if args.input.starts_with("http://") || args.input.starts_with("https://") {
        handle.set_source_url(args.input).await?;
    }
    done.wait_for(|v| *v).await.context("Session ended")?;
    let snapshot = printer.state.lock().unwrap().clone();
    let result = finish(
        &handle,
        snapshot,
        &path,
        &target,
        &settings,
        args.formats,
        args.out_dir,
        start,
    )
    .await;
    let closed = handle.shutdown().await;
    shared.shutdown().await;
    let summary = result?;
    closed?;
    if args.json_progress {
        println!("{summary}")
    } else {
        eprintln!();
        println!("{}", serde_json::to_string_pretty(&summary)?)
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn finish(
    handle: &hlp_pipeline::SessionHandle,
    snapshot: Snapshot,
    path: &Path,
    target: &str,
    settings: &Settings,
    formats: Vec<String>,
    out_dir: Option<PathBuf>,
    start: Instant,
) -> Result<serde_json::Value> {
    if let Some((code, detail)) = snapshot.error {
        return Err(anyhow!(detail).context(crate::ExitCode(code)));
    }
    let status = snapshot.status.context("Missing final status")?;
    if status.stage == "needs-setup" {
        return Err(
            anyhow!("Run hlp-cli setup to install the missing components")
                .context(crate::ExitCode(2)),
        );
    }
    if status.stage != "complete" || status.utt_failed > 0 {
        return Err(anyhow!("Subtitle generation failed").context(crate::ExitCode(4)));
    }
    let lang = snapshot.language.context("Missing language event")?;
    let source = lang.detected.as_ref().map_or("und", |l| l.0.as_str());
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let out = out_dir.unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_owned());
    let mut variants = vec![(source.to_owned(), SubtitleVariant::Original)];
    if target != "none" && target != source {
        variants.push((target.to_owned(), SubtitleVariant::Translation));
        variants.push((
            format!("{source}-{target}"),
            SubtitleVariant::Bilingual(settings.bilingual_order.clone()),
        ));
    }
    let mut outputs = Vec::new();
    for name in formats {
        let format = match name.as_str() {
            "srt" => SubtitleFormat::Srt,
            "vtt" => SubtitleFormat::Vtt,
            _ => SubtitleFormat::Ass,
        };
        for (code, variant) in &variants {
            let path = out.join(format!("{stem}.{code}.{name}"));
            handle
                .send(SessionCommand::Export {
                    format: format.clone(),
                    variant: variant.clone(),
                    path: path.clone(),
                })
                .await?;
            outputs.push(path);
        }
    }
    let (asr, mt) = handle.request_counts();
    Ok(json!({
        "event":"summary","detected":lang.detected,"confidence":lang.confidence,
        "utterances":status.utt_total,"done":status.utt_done,"noSpeech":status.utt_no_speech,"failed":status.utt_failed,
        "translated":status.translated_count,"asrRtf":status.asr_rtf,"msPerTranslation":status.ms_per_translation,
        "wallSeconds":start.elapsed().as_secs_f64(),"asrRequests":asr,"translationRequests":mt,"outputs":outputs
    }))
}
