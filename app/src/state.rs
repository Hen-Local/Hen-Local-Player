use crate::events::Bridge;
use anyhow::{Context, Result};
use hlp_core::{settings::Settings, types::*};
use hlp_pipeline::{store, AsrClient, Session, SessionHandle, SharedEngines};
use parking_lot::Mutex as SyncMutex;
use std::{path::PathBuf, sync::Arc};
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct OpenFile {
    pub path: PathBuf,
    pub session: SessionHandle,
    pub events: Arc<Bridge>,
}
pub struct Player {
    pub settings: Settings,
    pub caps: Caps,
    pub engines: SharedEngines,
    pub active: Option<OpenFile>,
    pub ready: bool,
    pub pending: Vec<PathBuf>,
    #[cfg(target_os = "macos")]
    pub pending_menu: Option<String>,
}
impl Player {
    pub async fn open(
        &mut self,
        app: &AppHandle,
        path: PathBuf,
        url: Option<String>,
    ) -> Result<()> {
        // A renamed or duplicated file shares the same exclusive subtitle cache.
        // Otherwise keep the current session alive until its replacement opens.
        if let Some(current) = &self.active {
            let candidate = path.clone();
            let fingerprint =
                tokio::task::spawn_blocking(move || hlp_core::fingerprint::fingerprint(&candidate))
                    .await??;
            if current.session.fingerprint == fingerprint {
                self.close().await;
            }
        }
        let events = Arc::new(Bridge {
            app: app.clone(),
            source_path: path.clone(),
            language: Default::default(),
        });
        let session = Session::open_for_playback(
            path.clone(),
            self.settings.clone(),
            self.caps.clone(),
            events.clone(),
            self.engines.clone(),
        )
        .await?;
        if let Some(url) = url {
            if let Err(error) = session.set_source_url(url).await {
                let _ = session.shutdown().await;
                return Err(error);
            }
        }
        self.close().await;
        self.active = Some(OpenFile {
            path,
            session,
            events,
        });
        Ok(())
    }
    async fn close(&mut self) {
        if let Some(previous) = self.active.take() {
            if let Err(error) = previous.session.shutdown().await {
                tracing::warn!(%error, "Closing previous movie");
            }
        }
    }
    pub fn file(&self) -> Result<&OpenFile> {
        self.active.as_ref().context("Open a media file first")
    }
    pub async fn apply(&mut self, settings: Settings) -> Result<()> {
        validate(&settings)?;
        if let Some(file) = &self.active {
            file.session
                .send(SessionCommand::UpdateSettings(settings.clone()))
                .await?;
        } else if settings.asr_model != self.settings.asr_model {
            let mut client = self.engines.asr.lock().await;
            client.shutdown().await;
            *client = AsrClient::new(hlp_models::asr_model_dir(&settings.asr_model));
        }
        self.settings = settings;
        store::write_json(&store::data_root().join("settings.json"), &self.settings)
    }
}
fn validate(s: &Settings) -> Result<()> {
    use hlp_core::lang::lookup;
    anyhow::ensure!(
        matches!(
            s.accent_theme.as_str(),
            "neon-blue" | "neon-orange" | "neon-pink" | "neon-green"
        ),
        "Unknown accent theme"
    );
    anyhow::ensure!(
        matches!(s.source_language.as_str(), "auto" | "auto-per-line")
            || lookup(&s.source_language).is_some_and(|l| l.qwen3_asr_name.is_some()),
        "Unsupported spoken language"
    );
    anyhow::ensure!(
        s.target_language == "none" || lookup(&s.target_language).is_some_and(|l| l.hy_mt2_target),
        "Unsupported translation target"
    );
    anyhow::ensure!(
        if cfg!(windows) {
            s.asr_model == "sherpa-qwen3-asr-0.6b"
        } else {
            matches!(s.asr_model.as_str(), "qwen3-asr-1.7b" | "qwen3-asr-0.6b")
        },
        "Unsupported recognition model"
    );
    anyhow::ensure!(
        matches!(
            s.subtitle_mode.as_str(),
            "original" | "translation" | "both" | "off"
        ) && matches!(s.ui_language.as_str(), "system" | "en" | "zh"),
        "Invalid display preference"
    );
    anyhow::ensure!(
        (0.6..=2.).contains(&s.subtitle_scale)
            && (0.0..=0.8).contains(&s.subtitle_background)
            && (0.0..=0.4).contains(&s.subtitle_bottom_offset),
        "Invalid subtitle size or position"
    );
    anyhow::ensure!(
        s.lookahead_seconds > 0 && s.lookahead_seconds <= 86400 && s.cache_limit_gb > 0,
        "Invalid processing limit"
    );
    Ok(())
}
#[derive(Default)]
pub struct Activity(SyncMutex<Option<CancellationToken>>);
pub struct Job<'a> {
    activity: &'a Activity,
    pub cancel: CancellationToken,
}
impl Activity {
    pub fn begin(&self) -> Result<Job<'_>> {
        let mut job = self.0.lock();
        anyhow::ensure!(job.is_none(), "An operation is already in progress");
        let cancel = CancellationToken::new();
        *job = Some(cancel.clone());
        Ok(Job {
            activity: self,
            cancel,
        })
    }
    pub fn cancel(&self) {
        if let Some(token) = self.0.lock().as_ref() {
            token.cancel();
        }
    }
}
impl Drop for Job<'_> {
    fn drop(&mut self) {
        self.activity.0.lock().take();
    }
}
pub struct Desktop {
    pub player: Arc<Mutex<Player>>,
    pub url: Activity,
    pub setup: Activity,
    pub preview: SyncMutex<Option<(String, UrlInfo)>>,
}
impl Desktop {
    #[cfg(target_os = "macos")]
    pub fn window_closed(&self) {
        self.url.cancel();
        self.preview.lock().take();
        // Serialize closing behind any in-flight open and ahead of a Finder
        // reopen of the same file, which needs the previous session's cache lock.
        let mut player = tauri::async_runtime::block_on(self.player.clone().lock_owned());
        player.ready = false;
        player.pending.clear();
        player.pending_menu = None;
        tauri::async_runtime::spawn(async move { player.close().await });
    }

    pub async fn new() -> Result<Self> {
        let root = store::data_root();
        let settings: Settings = store::read(&root.join("settings.json"))
            .filter(|s| validate(s).is_ok())
            .unwrap_or_default();
        let engines = SharedEngines::new(&settings).await?;
        let pending = std::env::args_os()
            .skip(1)
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .collect();
        Ok(Self {
            player: Arc::new(Mutex::new(Player {
                settings,
                caps: Caps::new(),
                engines,
                active: None,
                pending,
                ready: false,
                #[cfg(target_os = "macos")]
                pending_menu: None,
            })),
            url: Default::default(),
            setup: Default::default(),
            preview: Default::default(),
        })
    }
}
