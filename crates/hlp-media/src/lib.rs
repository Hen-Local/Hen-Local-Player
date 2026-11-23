//! Tool-backed media I/O. No inference is performed here.
use hlp_core::types::*;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub yt_dlp: PathBuf,
    pub deno: PathBuf,
}
impl Tools {
    pub fn resolve() -> Self {
        todo!()
    }
}
pub fn probe(tools: &Tools, path: &Path) -> anyhow::Result<MediaInfo> {
    todo!()
}
pub fn default_caps(webview: &str) -> Caps {
    todo!()
}
pub fn decide(info: &MediaInfo, caps: &Caps, selected_audio: usize) -> PlaybackPlan {
    todo!()
}
/// Writes complete PCM frames to disk before invoking the callback with samples and extracted milliseconds.
pub fn extract_audio(
    tools: &Tools,
    input: &Path,
    track: &AudioStream,
    dialogue_focus: bool,
    pcm: &Path,
    on_frame: impl FnMut(&[f32], Ms) -> anyhow::Result<()>,
    cancel: CancellationToken,
) -> anyhow::Result<u64> {
    todo!()
}
pub fn prepare_playback(
    tools: &Tools,
    input: &Path,
    info: &MediaInfo,
    caps: &Caps,
    track: usize,
    plan: &PlaybackPlan,
    output: &Path,
    progress: impl FnMut(f32),
    cancel: CancellationToken,
) -> anyhow::Result<PathBuf> {
    todo!()
}
#[derive(Clone)]
pub struct MediaServer;
impl MediaServer {
    pub async fn start() -> anyhow::Result<Self> {
        todo!()
    }
    pub fn register(&self, path: &Path) -> anyhow::Result<String> {
        todo!()
    }
    pub fn unregister(&self, url: &str) {
        todo!()
    }
}
mod downloader;
pub use downloader::Downloader;
