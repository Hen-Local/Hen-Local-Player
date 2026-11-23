//! Tool-backed media I/O. Run synchronous probing and preparation on a blocking thread.
mod capabilities;
mod downloader;
mod ffmpeg;
mod probe;
mod process;
mod server;

pub use capabilities::{decide, default_caps};
pub use downloader::Downloader;
pub use ffmpeg::{extract_audio, prepare_playback};
pub use probe::probe;
pub use server::MediaServer;

use std::{env, path::PathBuf};

#[derive(Clone)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub yt_dlp: PathBuf,
    pub deno: PathBuf,
}

impl Tools {
    pub fn resolve() -> Self {
        Self {
            ffmpeg: resolve("ffmpeg", "HLP_FFMPEG"),
            ffprobe: resolve("ffprobe", "HLP_FFPROBE"),
            yt_dlp: resolve("yt-dlp", "HLP_YTDLP"),
            deno: resolve("deno", "HLP_DENO"),
        }
    }
}

fn resolve(name: &str, variable: &str) -> PathBuf {
    if let Some(path) = env::var_os(variable).filter(|v| !v.is_empty()) {
        return PathBuf::from(path);
    }
    let managed = hlp_models::tool_path(name);
    if managed.is_file() || env::var_os("HLP_ALLOW_SYSTEM_TOOLS").as_deref() != Some("1".as_ref()) {
        return managed;
    }
    let binary = format!("{name}{}", env::consts::EXE_SUFFIX);
    env::var_os("PATH")
        .and_then(|path| {
            env::split_paths(&path)
                .map(|dir| dir.join(&binary))
                .find(|file| file.is_file())
        })
        .unwrap_or(managed)
}
