use crate::Tools;
use hlp_core::types::*;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
pub struct Downloader {
    tools: Tools,
}
impl Downloader {
    pub fn new(tools: Tools) -> Self {
        Self { tools }
    }
    pub async fn probe(&self, url: &str, cancel: CancellationToken) -> anyhow::Result<UrlInfo> {
        todo!()
    }
    pub async fn download(
        &self,
        url: &str,
        info: &UrlInfo,
        dir: &Path,
        progress: impl Fn(DownloadProgress) + Send + Sync,
        cancel: CancellationToken,
    ) -> anyhow::Result<PathBuf> {
        todo!()
    }
}
