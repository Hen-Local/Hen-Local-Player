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
use std::{path::PathBuf, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    task::JoinHandle,
};
pub use translate::Translator;

#[derive(Clone)]
pub struct SharedEngines {
    pub asr: Arc<Mutex<AsrClient>>,
    pub translator: Arc<Mutex<Translator>>,
    pub server: MediaServer,
}
impl SharedEngines {
    pub async fn new(settings: &Settings) -> Result<Self> {
        Ok(Self {
            asr: Arc::new(Mutex::new(AsrClient::new(hlp_models::asr_model_dir(
                &settings.asr_model,
            )))),
            translator: Arc::new(Mutex::new(Translator::new())),
            server: MediaServer::start().await?,
        })
    }
    pub async fn shutdown(&self) {
        self.asr.lock().await.shutdown().await;
        *self.translator.lock().await = Translator::new();
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
