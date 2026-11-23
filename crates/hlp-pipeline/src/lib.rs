pub mod asr_client;
pub mod translate;
pub use asr_client::AsrClient;
use hlp_core::{events::PipelineEvents, settings::Settings, types::*};
use hlp_media::MediaServer;
use std::{path::PathBuf, sync::Arc};
pub use translate::Translator;
#[derive(Clone)]
pub struct SharedEngines {
    pub asr: Arc<tokio::sync::Mutex<AsrClient>>,
    pub translator: Arc<tokio::sync::Mutex<Translator>>,
    pub server: MediaServer,
}
impl SharedEngines {
    pub async fn new(settings: &Settings) -> anyhow::Result<Self> {
        todo!()
    }
}
pub struct Session;
pub struct SessionHandle {
    pub id: String,
    pub fingerprint: String,
}
impl Session {
    pub async fn open(
        input_path: PathBuf,
        settings: Settings,
        caps: Caps,
        events: Arc<dyn PipelineEvents>,
        shared: SharedEngines,
    ) -> anyhow::Result<SessionHandle> {
        todo!()
    }
}
impl SessionHandle {
    pub async fn send(&self, command: SessionCommand) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn shutdown(self) -> anyhow::Result<()> {
        todo!()
    }
}
