//! Shared component installation and discovery.
use hlp_core::types::ComponentStatus;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
pub fn ominix_root() -> PathBuf {
    todo!()
}
pub fn model_root() -> PathBuf {
    todo!()
}
pub fn asr_model_dir(model: &str) -> PathBuf {
    todo!()
}
pub fn llama_server_path() -> PathBuf {
    todo!()
}
pub fn gguf_path() -> PathBuf {
    todo!()
}
pub fn vad_path() -> PathBuf {
    todo!()
}
pub fn tool_path(name: &str) -> PathBuf {
    todo!()
}
pub fn status() -> Vec<ComponentStatus> {
    todo!()
}
pub fn required_ids(include_url_tools: bool) -> Vec<String> {
    todo!()
}
pub async fn download(
    ids: &[String],
    progress: impl Fn(ComponentStatus) + Send + Sync,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    todo!()
}
pub async fn update_ytdlp(
    progress: impl Fn(ComponentStatus) + Send + Sync,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    todo!()
}
