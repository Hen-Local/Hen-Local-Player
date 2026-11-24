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
    // A bare CLI filename has an empty parent. Keep its relative spelling in
    // the playlist so the open item and the directory entry share an identity.
    let directory = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    for entry in std::fs::read_dir(directory)
        .context("Read the selected folder")?
        .flatten()
    {
        let name = entry.file_name();
        if name.to_str().is_none() {
            continue;
        }
        let path = path.join(name);
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

#[derive(Serialize)]
pub struct FolderMedia {
    folder: PathBuf,
    files: Vec<PathBuf>,
}

fn siblings(source: &Path) -> Result<FolderMedia> {
    let folder = source.parent().context("Media has no folder")?;
    let mut files = folder_files(folder)?;
    files.retain(|p| {
        !p.file_name()
            .is_some_and(|s| s.to_string_lossy().starts_with('.'))
    });
    // Keep the spelling from the opener as the active row's identity on the UI.
    let exact = files.iter().position(|file| file == source);
    let selected = exact.or_else(|| {
        let name = source.file_name()?.to_str()?;
        files.iter().position(|file| {
            file.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| name.eq_ignore_ascii_case(s))
        })
    });
    if let Some(index) = selected {
        files[index] = source.to_owned();
    } else {
        // All Files can open hidden media or a playable file with an unusual extension.
        files.push(source.to_owned());
    }
    let directory = if folder.as_os_str().is_empty() {
        Path::new(".")
    } else {
        folder
    };
    Ok(FolderMedia {
        folder: std::path::absolute(directory)?,
        files,
    })
}

#[tauri::command]
pub async fn list_media_siblings(
    state: State<'_, Desktop>,
    path: PathBuf,
) -> Result<Option<FolderMedia>, String> {
    {
        let player = state.player.lock().await;
        if !player.active.as_ref().is_some_and(|file| file.path == path) {
            return Ok(None);
        }
    }
    tauri::async_runtime::spawn_blocking(move || siblings(&path))
        .await
        .map_err(|e| e.to_string())?
        .map(Some)
        .map_err(|e| format!("{e:#}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sibling_playlist_lists_playable_neighbors_and_keeps_the_open_file() {
        let dir =
            std::env::temp_dir().join(format!("hlp-playlist-siblings-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join("subfolder.mkv")).unwrap();
        for name in [
            "01.mp4",
            "02.MOV",
            "音乐.flac",
            "01.srt",
            "notes.txt",
            "._01.mp4",
            "subfolder.mkv/nested.mp4",
            "odd-extension",
        ] {
            fs::write(dir.join(name), []).unwrap();
        }
        let folder = siblings(&dir.join("02.MOV")).unwrap();
        assert_eq!(folder.folder, dir);
        let mut files = folder.files;
        files.sort();
        assert_eq!(
            files,
            ["01.mp4", "02.MOV", "音乐.flac"].map(|s| dir.join(s))
        );
        let alternate = dir.join("02.mov");
        if alternate.is_file() {
            let files = siblings(&alternate).unwrap().files;
            assert_eq!(files.len(), 3);
            assert!(files.contains(&alternate));
        }
        let unusual = dir.join("odd-extension");
        assert!(siblings(&unusual).unwrap().files.contains(&unusual));
        fs::remove_dir_all(dir).unwrap();
    }
}
