//! Shared component installation and discovery.
//! Paths and GGUF readiness adapted from Hen-Local-Translator/hen-local-models/src/lib.rs.
mod archive;
mod install;
mod manifest;
mod transfer;
mod update;
use hlp_core::types::{ComponentState, ComponentStatus};
use manifest::{catalog, component, Check, Component};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{LazyLock, Mutex, OnceLock},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub fn ominix_root() -> PathBuf {
    std::env::var_os("HLP_OMINIX_ROOT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".OminiX")
        })
}
pub fn model_root() -> PathBuf {
    ominix_root().join("models")
}
pub fn asr_model_dir(model: &str) -> PathBuf {
    let name = match model {
        "sherpa-qwen3-asr-0.6b" | "asr-qwen3-0.6b-sherpa" => {
            "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25"
        }
        "qwen3-asr-0.6b" | "asr-qwen3-0.6b-mlx" => "qwen3-asr-0.6b",
        _ if cfg!(windows) => "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25",
        _ => "qwen3-asr-1.7b",
    };
    model_root().join(name)
}
fn executable(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}
pub fn llama_server_path() -> PathBuf {
    ominix_root()
        .join("llama.cpp/b11429")
        .join(executable("llama-server"))
}
pub fn gguf_path() -> PathBuf {
    model_root().join("Hy-MT2-1.8B-Q4_K_M.gguf")
}
static BUNDLED_VAD: OnceLock<PathBuf> = OnceLock::new();

pub fn use_bundled_vad(path: &Path) {
    if let Some(c) = component("vad-silero") {
        if verified_single_file(path, c) {
            let _ = BUNDLED_VAD.set(path.to_owned());
        }
    }
}

pub fn vad_path() -> PathBuf {
    BUNDLED_VAD
        .get()
        .cloned()
        .unwrap_or_else(|| model_root().join("silero-vad/silero_vad.onnx"))
}
// Only the desktop shell opts into its signed, adjacent playback executables.
// Installer destinations and the CLI continue to use their own component root.
static BUNDLED_PLAYBACK_TOOLS: OnceLock<PathBuf> = OnceLock::new();

pub fn use_bundled_playback_tools() {
    if let Ok(app) = std::env::current_exe() {
        if let Some(dir) = app.parent() {
            if ["ffmpeg", "ffprobe"]
                .iter()
                .all(|name| dir.join(executable(name)).is_file())
            {
                let _ = BUNDLED_PLAYBACK_TOOLS.set(dir.to_path_buf());
            }
        }
    }
}
pub fn tool_path(name: &str) -> PathBuf {
    if matches!(name, "ffmpeg" | "ffprobe") {
        if let Some(dir) = BUNDLED_PLAYBACK_TOOLS.get() {
            return dir.join(executable(name));
        }
    }
    let id = if name == "ffprobe" { "ffmpeg" } else { name };
    let root = ominix_root();
    component(id)
        .filter(|c| c.commands.iter().any(|n| n == name))
        .map(|c| active_directory(&root, c).join(executable(name)))
        .unwrap_or_else(|| root.join("tools/unavailable"))
}
fn active_directory(root: &Path, c: &Component) -> PathBuf {
    if c.id == "yt-dlp" {
        let parent = root.join("tools/yt-dlp");
        if let Ok(version) = fs::read_to_string(parent.join("current")) {
            let version = version.trim();
            let dir = parent.join(version);
            if update::valid_version(version) && dir.join(executable("yt-dlp")).is_file() {
                return dir;
            }
        }
    }
    root.join(&c.directory)
}
fn version_ok(path: &Path, name: &str) -> bool {
    if !path.is_file() {
        return false;
    }
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    if name == "yt-dlp" {
        cmd.arg("--ignore-config");
    }
    let flag = if matches!(name, "ffmpeg" | "ffprobe") {
        "-version"
    } else {
        "--version"
    };
    let Ok(mut child) = cmd
        .arg(flag)
        .env("DENO_NO_UPDATE_CHECK", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(15)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}
fn file_len(p: &Path) -> u64 {
    p.metadata()
        .ok()
        .filter(|m| m.is_file())
        .map_or(0, |m| m.len())
}
fn verified_single_file(path: &Path, c: &Component) -> bool {
    use sha2::{Digest, Sha256};
    let first = &c.files[0];
    file_len(path) == first.bytes
        && fs::read(path).is_ok_and(|bytes| hex::encode(Sha256::digest(bytes)) == first.sha256)
}

pub fn asr_component_id(model: &str) -> &'static str {
    match model {
        "qwen3-asr-0.6b" => "asr-qwen3-0.6b-mlx",
        "sherpa-qwen3-asr-0.6b" => "asr-qwen3-0.6b-sherpa",
        _ => "asr-qwen3-1.7b-mlx",
    }
}

/// Check only the user's selected engines. URL helpers and alternate ASR
/// models have no bearing on transcribing local audio.
pub fn missing_for_transcription(model: &str, translate: bool) -> Vec<String> {
    let root = ominix_root();
    let mut ids = vec!["ffmpeg", "vad-silero", asr_component_id(model)];
    if translate {
        ids.extend(["mt-hy-mt2-1.8b", "llama-runtime-b11429"]);
    }
    ids.into_iter()
        .filter(|id| !component(id).is_some_and(|c| ready(&root, c)))
        .map(str::to_owned)
        .collect()
}

fn ready(root: &Path, c: &Component) -> bool {
    let dir = if c.id == "ffmpeg" {
        BUNDLED_PLAYBACK_TOOLS
            .get()
            .cloned()
            .unwrap_or_else(|| active_directory(root, c))
    } else {
        active_directory(root, c)
    };
    let first = &c.files[0];
    match c.ready {
        Check::Mlx => {
            let marker = fs::read(dir.join(".moxin-model-complete.json"))
                .ok()
                .and_then(|body| serde_json::from_slice::<serde_json::Value>(&body).ok());
            let marked = marker
                .as_ref()
                .and_then(|m| m.get("repo_id"))
                .and_then(|v| v.as_str())
                == c.repo.as_deref();
            c.files.iter().all(|f| {
                let name = f.name.as_deref().expect("raw filename");
                // Legacy installations sometimes omit the model card.
                (marked && name == "README.md") || file_len(&dir.join(name)) == f.bytes
            })
        }
        Check::File => {
            if c.id == "vad-silero" && BUNDLED_VAD.get().is_some() {
                return true;
            }
            verified_single_file(&dir.join(first.name.as_ref().expect("raw filename")), c)
        }
        Check::Sherpa => [
            "conv_frontend.onnx",
            "encoder.int8.onnx",
            "decoder.int8.onnx",
            "tokenizer/vocab.json",
            "tokenizer/merges.txt",
            "tokenizer/tokenizer_config.json",
        ]
        .iter()
        .all(|p| file_len(&dir.join(p)) > 0),
        Check::Gguf => {
            let path = dir.join(first.name.as_ref().expect("GGUF filename"));
            let mut magic = [0; 4];
            file_len(&path) == first.bytes
                && File::open(&path)
                    .and_then(|mut f| f.read_exact(&mut magic))
                    .is_ok()
                && &magic == b"GGUF"
                && fs::read_to_string(path.with_extension("gguf.sha256"))
                    .is_ok_and(|s| s.trim() == first.sha256)
        }
        Check::Tool => c
            .commands
            .iter()
            .all(|n| version_ok(&dir.join(executable(n)), n)),
    }
}
fn event(c: &Component, state: ComponentState, bytes_done: u64) -> ComponentStatus {
    ComponentStatus {
        id: c.id.clone(),
        ready: state == ComponentState::Ready,
        bytes_total: c.total(),
        bytes_done,
        state,
    }
}
// A status query during setup sees the same byte count or error as subscribers.
// Installed files remain the source of truth after the transfer finishes.
type Activity = HashMap<(PathBuf, String), ComponentStatus>;
static ACTIVITY: LazyLock<Mutex<Activity>> = LazyLock::new(Mutex::default);
fn record(root: &Path, item: &ComponentStatus) {
    let key = (root.to_owned(), item.id.clone());
    let mut activity = ACTIVITY.lock().unwrap_or_else(|e| e.into_inner());
    if item.ready {
        activity.remove(&key);
    } else {
        activity.insert(key, item.clone());
    }
}
pub fn status() -> Vec<ComponentStatus> {
    let root = ominix_root();
    catalog()
        .map(|c| {
            let recent = ACTIVITY
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&(root.clone(), c.id.clone()))
                .cloned();
            if let Some(active) = recent.as_ref().filter(|s| {
                matches!(
                    s.state,
                    ComponentState::Downloading | ComponentState::Verifying
                )
            }) {
                return active.clone();
            }
            if ready(&root, c) {
                event(c, ComponentState::Ready, c.total())
            } else if let Some(failure) = recent {
                failure
            } else {
                let done = c
                    .files
                    .iter()
                    .map(|f| file_len(&transfer::part_path(&c.stage(&root), f)).min(f.bytes))
                    .sum();
                event(c, ComponentState::Missing, done)
            }
        })
        .collect()
}
pub fn required_ids(include_url_tools: bool) -> Vec<String> {
    catalog()
        .filter(|c| c.group == "subtitles" || include_url_tools && c.group == "urls")
        .map(|c| c.id.clone())
        .collect()
}
pub async fn download(
    ids: &[String],
    progress: impl Fn(ComponentStatus) + Send + Sync,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let root = ominix_root();
    let client = transfer::client()?;
    for id in ids {
        let c = component(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown or unavailable component: {id}"))?;
        let report = |event: ComponentStatus| {
            record(&root, &event);
            progress(event);
        };
        if let Err(err) = install::component(&root, c, &client, &report, &cancel).await {
            let done = c
                .files
                .iter()
                .map(|a| file_len(&transfer::part_path(&c.stage(&root), a)).min(a.bytes))
                .sum();
            report(event(c, ComponentState::Failed(format!("{err:#}")), done));
            return Err(err);
        }
    }
    Ok(())
}
pub async fn update_ytdlp(
    progress: impl Fn(ComponentStatus) + Send + Sync,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let root = ominix_root();
    let report = |event: ComponentStatus| {
        record(&root, &event);
        progress(event);
    };
    let result = update::update(&root, &report, &cancel).await;
    if let (Some(c), Err(err)) = (component("yt-dlp"), &result) {
        report(event(c, ComponentState::Failed(format!("{err:#}")), 0));
    }
    result
}

#[cfg(test)]
mod tests;
