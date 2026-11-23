use hlp_core::asr_protocol::{Hello, Transcription};
use std::path::{Path, PathBuf};
pub struct AsrClient;
impl AsrClient {
    pub fn new(model_dir: PathBuf) -> Self {
        todo!()
    }
    pub async fn load(&mut self) -> anyhow::Result<Hello> {
        todo!()
    }
    pub async fn transcribe(
        &mut self,
        pcm: &Path,
        start_sample: u64,
        end_sample: u64,
        language: Option<&str>,
        max_tokens: u32,
    ) -> anyhow::Result<Transcription> {
        todo!()
    }
    pub async fn unload(&mut self) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn shutdown(&mut self) {
        todo!()
    }
}
