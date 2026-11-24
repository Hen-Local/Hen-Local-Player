//! One session owns all state for one media file. Engines and the HTTP listener survive file changes.
pub mod asr_client;
mod session;
pub mod store;
mod timeline;
pub mod translate;
mod vad;
use anyhow::{Context, Result};
pub use asr_client::AsrClient;
use hlp_core::{events::*, settings::Settings, types::*};
use hlp_media::{MediaServer, Tools};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, oneshot, watch, Mutex},
    task::JoinHandle,
};
pub use translate::Translator;

#[derive(Clone)]
pub struct SharedEngines {
    pub asr: Arc<Mutex<AsrClient>>,
    pub translator: Arc<Mutex<Translator>>,
    pub server: MediaServer,
    model_users: watch::Sender<usize>,
}

/// Keep the models warm across individual utterances and overlapping sessions.
struct ModelUse(watch::Sender<usize>);

impl Drop for ModelUse {
    fn drop(&mut self) {
        self.0.send_modify(|users| *users -= 1);
    }
}

impl SharedEngines {
    pub async fn new(settings: &Settings) -> Result<Self> {
        Self::with_idle_grace(settings, Duration::from_secs(30)).await
    }
    async fn with_idle_grace(settings: &Settings, grace: Duration) -> Result<Self> {
        let (model_users, changes) = watch::channel(0);
        let engines = Self {
            asr: Arc::new(Mutex::new(AsrClient::new(hlp_models::asr_model_dir(
                &settings.asr_model,
            )))),
            translator: Arc::new(Mutex::new(Translator::new())),
            server: MediaServer::start().await?,
            model_users,
        };
        tokio::spawn(unload_idle_models(
            changes,
            grace,
            engines.asr.clone(),
            engines.translator.clone(),
        ));
        Ok(engines)
    }
    fn use_models(&self) -> ModelUse {
        self.model_users.send_modify(|users| *users += 1);
        ModelUse(self.model_users.clone())
    }
    pub async fn shutdown(&self) {
        self.asr.lock().await.shutdown().await;
        self.translator.lock().await.shutdown().await;
    }
}

async fn unload_idle_models(
    mut users: watch::Receiver<usize>,
    grace: Duration,
    asr: Arc<Mutex<AsrClient>>,
    mt: Arc<Mutex<Translator>>,
) {
    while users.changed().await.is_ok() {
        while *users.borrow_and_update() == 0 {
            match tokio::time::timeout(grace, users.changed()).await {
                Ok(Ok(())) => continue,
                Ok(Err(_)) => return,
                Err(_) => {
                    let (mut asr, mut mt) = tokio::join!(asr.lock(), mt.lock());
                    // Another session may have acquired the models while we
                    // waited for a cancelled request to return its lock.
                    if matches!(users.has_changed(), Ok(false)) {
                        tokio::join!(asr.shutdown(), mt.shutdown());
                    }
                    break;
                }
            }
        }
    }
}
pub struct Session;
enum Message {
    Command(SessionCommand),
    SourceUrl(String),
}
struct Request {
    action: Message,
    reply: oneshot::Sender<Result<()>>,
}
#[derive(Default)]
struct Counts {
    asr: std::sync::atomic::AtomicU64,
    translation: std::sync::atomic::AtomicU64,
}
pub struct SessionHandle {
    counts: Arc<Counts>,
    pub id: String,
    pub fingerprint: String,
    tx: mpsc::Sender<Request>,
    task: JoinHandle<()>,
}
impl SessionHandle {
    pub fn request_counts(&self) -> (u64, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (
            self.counts.asr.load(Relaxed),
            self.counts.translation.load(Relaxed),
        )
    }
    async fn request(&self, action: Message) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Request { action, reply })
            .await
            .map_err(|_| anyhow::anyhow!("Session closed"))?;
        rx.await.context("Session closed before responding")?
    }
    pub async fn send(&self, command: SessionCommand) -> Result<()> {
        self.request(Message::Command(command)).await
    }
    pub async fn set_source_url(&self, url: String) -> Result<()> {
        self.request(Message::SourceUrl(url)).await
    }
    pub async fn shutdown(self) -> Result<()> {
        let result = self.send(SessionCommand::Shutdown).await;
        self.task.await?;
        result
    }
}
impl Session {
    pub async fn open(
        path: PathBuf,
        settings: Settings,
        caps: Caps,
        events: Arc<dyn PipelineEvents>,
        shared: SharedEngines,
    ) -> Result<SessionHandle> {
        Self::open_in(path, settings, caps, events, shared, store::data_root()).await
    }
    pub async fn open_in(
        path: PathBuf,
        settings: Settings,
        caps: Caps,
        events: Arc<dyn PipelineEvents>,
        shared: SharedEngines,
        root: PathBuf,
    ) -> Result<SessionHandle> {
        Self::open_with_mode(path, settings, caps, events, shared, root, true).await
    }
    /// Desktop playback restores cached subtitles immediately. New speech work
    /// begins when the viewer explicitly requests it for the selected audio track.
    pub async fn open_for_playback(
        path: PathBuf,
        settings: Settings,
        caps: Caps,
        events: Arc<dyn PipelineEvents>,
        shared: SharedEngines,
    ) -> Result<SessionHandle> {
        Self::open_with_mode(
            path,
            settings,
            caps,
            events,
            shared,
            store::data_root(),
            false,
        )
        .await
    }
    async fn open_with_mode(
        path: PathBuf,
        settings: Settings,
        caps: Caps,
        events: Arc<dyn PipelineEvents>,
        shared: SharedEngines,
        root: PathBuf,
        automatic_transcription: bool,
    ) -> Result<SessionHandle> {
        timeline::validate_settings(&settings)?;
        let tools = Tools::resolve();
        let probe_tools = tools.clone();
        let data = root.clone();
        let (path, fp, old, probe, guard) = tokio::task::spawn_blocking(move || -> Result<_> {
            let path = path.canonicalize().context("Open media file")?;
            let fp = hlp_core::fingerprint::fingerprint(&path)?;
            let guard = store::lock_project(&data, &fp)?;
            let old =
                store::read::<store::Project>(&data.join("media").join(&fp).join("project.json"))
                    .filter(|p| p.schema_version == 1 && p.fingerprint == fp);
            let probe = if let Some(p) = old
                .as_ref()
                .filter(|p| p.probe.video.is_some() || !p.probe.audio.is_empty())
            {
                p.probe.clone()
            } else if probe_tools.ffprobe.is_file() {
                hlp_media::probe(&probe_tools, &path)?
            } else {
                MediaInfo {
                    container: String::new(),
                    duration_ms: 0,
                    video: None,
                    audio: vec![],
                    has_subtitle_streams: false,
                }
            };
            Ok((path, fp, old, probe, guard))
        })
        .await??;
        let id = format!("{:032x}", rand::random::<u128>());
        let language = timeline::initial_language(&id, &settings.source_language);
        let default = probe
            .audio
            .iter()
            .find(|a| a.default)
            .or(probe.audio.first())
            .map_or(0, |a| a.order);
        let order = old
            .as_ref()
            .map(|p| p.audio_track_order)
            .filter(|&i| probe.audio.iter().any(|a| a.order == i))
            .unwrap_or(default);
        let plan = hlp_media::decide(&probe, &caps, order);
        let mut project = store::Project {
            schema_version: 1,
            fingerprint: fp.clone(),
            title: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            source_path: path,
            source_url: None,
            duration_ms: probe.duration_ms,
            probe,
            audio_track_order: order,
            playback: store::Playback {
                plan: plan.clone(),
                playable_path: None,
            },
            language,
            created_at: store::now(),
            updated_at: store::now(),
        };
        if let Some(old) = old {
            project.source_url = old.source_url;
            project.created_at = old.created_at;
            if old.audio_track_order == order && old.playback.plan == plan {
                project.playback = old.playback;
            }
        }
        let (tx, rx) = mpsc::channel(64);
        let counts = Arc::new(Counts::default());
        let mut ctx = session::Context {
            counts: counts.clone(),
            automatic_transcription,
            root,
            project,
            settings,
            caps,
            events,
            shared,
            tools,
            id: id.clone(),
            opened_at: store::now(),
            playhead: 0,
            playing: false,
        };
        let task = tokio::spawn(async move {
            let _guard = guard;
            if let Err(error) = session::serve(&mut ctx, rx).await {
                ctx.events.error(ErrorEvent {
                    session_id: Some(ctx.id.clone()),
                    code: "Session".into(),
                    message: format!("{error:#}"),
                    recoverable: false,
                });
            }
        });
        Ok(SessionHandle {
            id,
            fingerprint: fp,
            tx,
            task,
            counts,
        })
    }
}

#[cfg(test)]
mod playback_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::{
        sync::watch,
        time::{timeout, Duration},
    };

    struct Events {
        opened: AtomicUsize,
        status: watch::Sender<Option<PipelineStatus>>,
    }
    impl PipelineEvents for Events {
        fn media_opened(&self, _: MediaOpened) {
            self.opened.fetch_add(1, Ordering::Relaxed);
        }
        fn playback_prep(&self, _: PlaybackPrep) {}
        fn language(&self, _: LanguageEvent) {}
        fn cues(&self, _: CueBatch) {}
        fn error(&self, e: ErrorEvent) {
            eprintln!("{}: {}", e.code, e.message);
        }
        fn status(&self, s: PipelineStatus) {
            self.status.send_replace(Some(s));
        }
    }

    async fn wait_for(
        rx: &mut watch::Receiver<Option<PipelineStatus>>,
        stage: &str,
    ) -> Result<PipelineStatus> {
        let result = timeout(
            Duration::from_secs(90),
            rx.wait_for(|item| {
                item.as_ref()
                    .is_some_and(|s| s.stage == stage || s.stage == "error")
            }),
        )
        .await??;
        let status = result.as_ref().unwrap();
        anyhow::ensure!(status.stage == stage, "Unexpected stage: {}", status.stage);
        Ok(status.clone())
    }

    #[tokio::test]
    #[ignore = "requires the Japanese speech fixture, native playback tools and installed ASR worker/model"]
    async fn explicit_generation_starts_once_and_restores_the_result() -> Result<()> {
        let root = tempfile::tempdir()?;
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/fixture_ja.mp4");
        let settings = Settings {
            target_language: "none".into(),
            ..Settings::default()
        };
        let engines = SharedEngines::new(&settings).await?;
        let (status, mut events) = watch::channel(None);
        let sink = Arc::new(Events {
            opened: AtomicUsize::new(0),
            status,
        });
        let open = || {
            Session::open_with_mode(
                fixture.clone(),
                settings.clone(),
                hlp_media::default_caps("wkwebview"),
                sink.clone(),
                engines.clone(),
                root.path().to_owned(),
                false,
            )
        };

        let h = open().await?;
        wait_for(&mut events, "idle").await?;
        assert_eq!(h.request_counts(), (0, 0));
        let track = root
            .path()
            .join("media")
            .join(&h.fingerprint)
            .join("track-0");
        assert!(!track.join("audio.pcm").exists());
        h.send(SessionCommand::StartTranscription {
            components_ready: false,
            regenerate: false,
            audio_track_order: None,
        })
        .await?;
        wait_for(&mut events, "needs-setup").await?;
        assert!(!track.join("generate.requested").exists());
        for _ in 0..2 {
            h.send(SessionCommand::StartTranscription {
                components_ready: true,
                regenerate: false,
                audio_track_order: None,
            })
            .await?;
        }
        let done = wait_for(&mut events, "complete").await?;
        assert_eq!(done.utt_done, 6);
        assert_eq!(h.request_counts(), (6, 0));
        assert_eq!(
            sink.opened.load(Ordering::Relaxed),
            1,
            "generation must preserve the playing movie"
        );
        h.shutdown().await?;

        sink.status.send_replace(None);
        let restored = open().await?;
        assert_eq!(wait_for(&mut events, "complete").await?.utt_done, 6);
        assert_eq!(restored.request_counts(), (0, 0));
        restored.shutdown().await?;
        engines.shutdown().await;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires the Japanese fixture, native ASR and translation models"]
    async fn completed_track_schedules_a_new_translation_and_respects_lookahead() -> Result<()> {
        let root = tempfile::tempdir()?;
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/fixture_ja.mp4");
        let mut settings = Settings {
            target_language: "none".into(),
            ..Settings::default()
        };
        let grace = Duration::from_secs(1);
        let engines = SharedEngines::with_idle_grace(&settings, grace).await?;
        let (status, mut events) = watch::channel(None);
        let sink = Arc::new(Events {
            opened: AtomicUsize::new(0),
            status,
        });
        let open = |settings| {
            Session::open_in(
                fixture.clone(),
                settings,
                hlp_media::default_caps("wkwebview"),
                sink.clone(),
                engines.clone(),
                root.path().into(),
            )
        };
        let session = open(settings.clone()).await?;
        assert_eq!(wait_for(&mut events, "complete").await?.utt_done, 6);
        assert_eq!(session.request_counts(), (6, 0));
        session
            .send(SessionCommand::Playhead {
                ms: 22_500,
                playing: false,
            })
            .await?;
        settings.process_whole_file = false;
        session
            .send(SessionCommand::UpdateSettings(settings.clone()))
            .await?;
        session.send(SessionCommand::SetTarget("en".into())).await?;
        assert_eq!(wait_for(&mut events, "paused").await?.translated_count, 0);
        assert_eq!(session.request_counts(), (6, 0));
        // A second recording using the shared engines keeps the same process
        // alive across the first recording's idle deadline.
        let other = engines.use_models();
        settings.target_language = "en".into();
        settings.process_whole_file = true;
        session
            .send(SessionCommand::UpdateSettings(settings.clone()))
            .await?;
        assert_eq!(wait_for(&mut events, "complete").await?.translated_count, 6);
        assert_eq!(session.request_counts(), (6, 6));
        let pid = engines.translator.lock().await.process_id().unwrap();
        tokio::time::sleep(grace * 2).await;
        assert_eq!(engines.translator.lock().await.process_id(), Some(pid));
        drop(other);
        timeout(Duration::from_secs(10), async {
            loop {
                let translation_stopped = engines.translator.lock().await.process_id().is_none();
                if translation_stopped && !engines.asr.lock().await.has_supervisor() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await?;
        // The later regeneration exercises cold reload in the same session.

        let track = root
            .path()
            .join("media")
            .join(&session.fingerprint)
            .join("track-0");
        let pcm = track.join("audio.pcm");
        let modified = std::fs::metadata(&pcm)?.modified()?;
        let regenerate = |order, ready| SessionCommand::StartTranscription {
            components_ready: ready,
            regenerate: true,
            audio_track_order: Some(order),
        };
        assert!(session.send(regenerate(1, true)).await.is_err());
        session.send(regenerate(0, false)).await?;
        assert_eq!(
            wait_for(&mut events, "needs-setup").await?.translated_count,
            6
        );
        assert_eq!(session.request_counts(), (6, 6));
        session.send(regenerate(0, true)).await?;
        session.send(regenerate(0, true)).await?;
        assert_eq!(wait_for(&mut events, "complete").await?.translated_count, 6);
        assert_eq!(
            session.request_counts(),
            (12, 12),
            "one complete second pass"
        );
        assert_eq!(std::fs::metadata(&pcm)?.modified()?, modified);
        assert_eq!(sink.opened.load(Ordering::Relaxed), 1);
        session.shutdown().await?;

        sink.status.send_replace(None);
        let restored = open(settings).await?;
        assert_eq!(wait_for(&mut events, "complete").await?.translated_count, 6);
        assert_eq!(restored.request_counts(), (0, 0));
        restored.shutdown().await?;
        engines.shutdown().await;
        Ok(())
    }
}
