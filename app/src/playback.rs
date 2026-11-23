use crate::state::Desktop;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::LazyLock,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

static MEDIA_FORMATS: LazyLock<Vec<String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../media-formats.json")).expect("media extensions")
});

fn folder_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path).context("Read the selected folder")? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| {
                MEDIA_FORMATS
                    .iter()
                    .any(|format| ext.eq_ignore_ascii_case(format))
            })
            && path.is_file()
        {
            files.push(path);
        }
    }
    Ok(files)
}

#[tauri::command]
pub fn get_sample_media(app: AppHandle) -> Option<PathBuf> {
    let resource = app.path().resource_dir().ok()?.join("demo/demo.mp4");
    if resource.is_file() {
        return Some(resource);
    }
    #[cfg(debug_assertions)]
    {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/public/demo/demo.mp4");
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[tauri::command]
pub async fn choose_media_folder(app: AppHandle) -> Result<Vec<PathBuf>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<_> {
        let Some(folder) = app.dialog().file().blocking_pick_folder() else {
            return Ok(Vec::new());
        };
        let files = folder_files(&folder.into_path()?)?;
        anyhow::ensure!(
            !files.is_empty(),
            "No playable files were found in this folder."
        );
        Ok(files)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    start_ms: u64,
    title: String,
}

fn chapters(path: PathBuf) -> Result<Vec<Chapter>> {
    #[derive(Deserialize)]
    struct Tags {
        #[serde(default)]
        title: String,
    }
    #[derive(Deserialize)]
    struct Entry {
        start_time: String,
        tags: Option<Tags>,
    }
    #[derive(Deserialize)]
    struct Probe {
        chapters: Vec<Entry>,
    }

    let mut cmd = Command::new(hlp_media::Tools::resolve().ffprobe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe,crypto",
            "-show_entries",
            "chapter=start_time:chapter_tags=title",
            "-of",
            "json",
            "-i",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .context("Read chapter information")?;
    anyhow::ensure!(
        output.status.success(),
        "Could not read chapter information."
    );
    let data: Probe = serde_json::from_slice(&output.stdout)?;
    let mut chapters: Vec<_> = data
        .chapters
        .into_iter()
        .filter_map(|c| {
            let seconds = c.start_time.parse::<f64>().ok()?;
            (seconds.is_finite() && seconds >= 0.).then(|| Chapter {
                start_ms: (seconds * 1000.).round() as u64,
                title: c.tags.map(|t| t.title).unwrap_or_default(),
            })
        })
        .collect();
    chapters.sort_by_key(|c| c.start_ms);
    Ok(chapters)
}

#[tauri::command]
pub async fn list_chapters(
    state: State<'_, Desktop>,
    session_id: String,
) -> Result<Vec<Chapter>, String> {
    let path = {
        let player = state.player.lock().await;
        let Some(file) = player
            .active
            .as_ref()
            .filter(|f| f.session.id == session_id)
        else {
            return Ok(Vec::new());
        };
        file.path.clone()
    };
    tauri::async_runtime::spawn_blocking(move || chapters(path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn save_video_frame(
    app: AppHandle,
    state: State<'_, Desktop>,
    session_id: String,
    ms: u64,
) -> Result<bool, String> {
    let source = {
        let p = state.player.lock().await;
        p.active
            .as_ref()
            .filter(|file| file.session.id == session_id)
            .ok_or("The movie has changed.")?
            .path
            .clone()
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<bool> {
        let stem = source.file_stem().unwrap_or_default().to_string_lossy();
        let suggested = format!("{stem}-{}m{:02}s.png", ms / 60000, ms / 1000 % 60);
        let Some(selected) = app
            .dialog()
            .file()
            .add_filter("PNG", &["png"])
            .set_file_name(suggested)
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let destination = selected.into_path()?;
        anyhow::ensure!(
            destination
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("png")),
            "Use a PNG filename."
        );
        let mut cmd = Command::new(hlp_media::Tools::resolve().ffmpeg);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let frame = cmd
            .args([
                "-v",
                "error",
                "-nostdin",
                "-protocol_whitelist",
                "file,pipe,crypto",
                "-ss",
            ])
            .arg(format!("{:.3}", ms as f64 / 1000.))
            .arg("-i")
            .arg(source)
            .args([
                "-map",
                "0:V:0",
                "-an",
                "-sn",
                "-frames:v",
                "1",
                "-c:v",
                "png",
                "-f",
                "image2pipe",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .output()
            .context("Read the current video frame")?;
        anyhow::ensure!(
            frame.status.success() && frame.stdout.starts_with(b"\x89PNG"),
            "Could not capture a video frame at this time."
        );
        hlp_pipeline::store::atomic_write(&destination, &frame.stdout)?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
