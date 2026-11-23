//! Read user-selected subtitle files and text streams from the original movie.
use crate::state::Desktop;
use anyhow::{ensure, Context, Result};
use hlp_media::Tools;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
};
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tokio::{io::AsyncReadExt, process::Command};

const MAX_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Serialize)]
pub struct SubtitleFile {
    name: String,
    text: String,
}

#[derive(Serialize)]
pub struct EmbeddedTrack {
    order: usize,
    default: bool,
    forced: bool,
    title: Option<String>,
    language: Option<String>,
}

fn read_file(path: &Path) -> Result<SubtitleFile> {
    let extension = path.extension().and_then(|v| v.to_str()).unwrap_or("");
    ensure!(
        extension.eq_ignore_ascii_case("srt") || extension.eq_ignore_ascii_case("vtt"),
        "Select an SRT or WebVTT subtitle file"
    );
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), "Select a subtitle file");
    ensure!(metadata.len() <= MAX_BYTES, "Subtitle file exceeds 16 MiB");
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "Subtitle file exceeds 16 MiB"
    );
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little_endian = bytes[0] == 0xff;
        ensure!(bytes.len() % 2 == 0, "Invalid UTF-16 subtitle file");
        let units: Vec<_> = bytes[2..]
            .chunks_exact(2)
            .map(|b| {
                if little_endian {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect();
        String::from_utf16(&units).context("Invalid UTF-16 subtitle file")?
    } else {
        String::from_utf8(bytes).context("Save subtitles as UTF-8 or UTF-16")?
    };
    Ok(SubtitleFile {
        name: path
            .file_name()
            .context("Missing subtitle filename")?
            .to_string_lossy()
            .into_owned(),
        text,
    })
}

#[tauri::command]
pub async fn choose_subtitle_file(window: WebviewWindow) -> Result<Option<SubtitleFile>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<_> {
        let file = window
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter("SRT / WebVTT", &["srt", "vtt"])
            .blocking_pick_file();
        file.map(|f| read_file(&f.into_path()?)).transpose()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

async fn source_path(state: &Desktop, session_id: &str) -> Result<PathBuf, String> {
    let player = state.player.lock().await;
    player
        .active
        .as_ref()
        .filter(|file| file.session.id == session_id)
        .map(|file| file.path.clone())
        .ok_or_else(|| "Media has changed".to_owned())
}

async fn tool_output(mut cmd: Command, limit: u64) -> Result<Vec<u8>> {
    cmd.stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let mut child = cmd.spawn()?;
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .context("Missing subtitle output")?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .await?;
    ensure!(
        bytes.len() as u64 <= limit,
        "Subtitle data exceeds the size limit"
    );
    ensure!(
        child.wait().await?.success(),
        "Could not read the movie's text subtitles"
    );
    Ok(bytes)
}

#[derive(Deserialize)]
struct Probe {
    streams: Vec<Stream>,
}
#[derive(Deserialize)]
struct Stream {
    codec_name: Option<String>,
    #[serde(default)]
    disposition: Disposition,
    #[serde(default)]
    tags: Tags,
}
#[derive(Default, Deserialize)]
struct Disposition {
    #[serde(default)]
    default: u8,
    #[serde(default)]
    forced: u8,
}
#[derive(Default, Deserialize)]
struct Tags {
    title: Option<String>,
    language: Option<String>,
}

async fn embedded_tracks(ffprobe: &Path, source: &Path) -> Result<Vec<EmbeddedTrack>> {
    let mut cmd = Command::new(ffprobe);
    cmd.args([
        "-v",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-select_streams",
        "s",
        "-show_entries",
        "stream=codec_name:stream_tags=title,language:stream_disposition=default,forced",
        "-of",
        "json",
        "-i",
    ])
    .arg(source);
    let report: Probe = serde_json::from_slice(&tool_output(cmd, 1024 * 1024).await?)?;
    Ok(report
        .streams
        .into_iter()
        .enumerate()
        .filter_map(|(order, stream)| {
            matches!(
                stream.codec_name.as_deref()?,
                "subrip"
                    | "webvtt"
                    | "mov_text"
                    | "ass"
                    | "ssa"
                    | "text"
                    | "microdvd"
                    | "sami"
                    | "subviewer"
                    | "subviewer1"
                    | "mpl2"
                    | "jacosub"
                    | "realtext"
                    | "vplayer"
            )
            .then_some(EmbeddedTrack {
                order,
                default: stream.disposition.default != 0,
                forced: stream.disposition.forced != 0,
                title: stream.tags.title,
                language: stream.tags.language.filter(|s| s != "und"),
            })
        })
        .collect())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleInventory {
    embedded: Vec<EmbeddedTrack>,
    sidecars: Vec<String>,
    exact_sidecar: Option<String>,
    probe_failed: bool,
}

fn matches_movie(movie: &Path, name: &str) -> bool {
    let Some(stem) = movie.file_stem().and_then(|n| n.to_str()) else {
        return false;
    };
    // Only a sibling of this exact movie, optionally followed by a language or SDH tag.
    if name.contains(['/', '\\', ':']) {
        return false;
    }
    #[cfg(windows)]
    let (name, stem) = (name.to_lowercase(), stem.to_lowercase());
    #[cfg(windows)]
    let (name, stem) = (name.as_str(), stem.as_str());
    let Some((base, ext)) = name.rsplit_once('.') else {
        return false;
    };
    (ext.eq_ignore_ascii_case("srt") || ext.eq_ignore_ascii_case("vtt"))
        && (base == stem
            || base
                .strip_prefix(stem)
                .is_some_and(|suffix| suffix.starts_with('.') && suffix.len() > 1))
}

fn sidecars(movie: &Path) -> Vec<String> {
    let Some(directory) = movie.parent().and_then(|p| std::fs::read_dir(p).ok()) else {
        return Vec::new();
    };
    let mut names: Vec<_> = directory
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            (matches_movie(movie, &name) && entry.file_type().is_ok_and(|f| f.is_file()))
                .then_some(name)
        })
        .collect();
    names.sort();
    names
}

fn read_sidecar(movie: &Path, name: &str) -> Result<String> {
    ensure!(
        matches_movie(movie, name),
        "Select a matching subtitle file"
    );
    let path = movie.with_file_name(name);
    ensure!(
        path.symlink_metadata()?.is_file(),
        "Select a regular subtitle file"
    );
    Ok(read_file(&path)?.text)
}

#[tauri::command]
pub async fn list_subtitle_tracks(
    state: State<'_, Desktop>,
    session_id: String,
) -> Result<SubtitleInventory, String> {
    let path = source_path(&state, &session_id).await?;
    let neighbor = path.clone();
    let nearby = tauri::async_runtime::spawn_blocking(move || sidecars(&neighbor));
    let ffprobe = Tools::resolve().ffprobe;
    let tracks = if ffprobe.is_file() {
        embedded_tracks(&ffprobe, &path).await
    } else {
        Ok(vec![])
    };
    let sidecars = nearby.await.map_err(|e| e.to_string())?;
    let exact_sidecar = sidecars
        .iter()
        .find(|name| {
            let candidate = Path::new(name).file_stem();
            if cfg!(windows) {
                candidate.and_then(|v| v.to_str()).map(str::to_lowercase)
                    == path
                        .file_stem()
                        .and_then(|v| v.to_str())
                        .map(str::to_lowercase)
            } else {
                candidate == path.file_stem()
            }
        })
        .cloned();
    Ok(SubtitleInventory {
        exact_sidecar,
        probe_failed: tracks.is_err(),
        embedded: tracks.unwrap_or_default(),
        sidecars,
    })
}

#[tauri::command]
pub async fn read_sidecar_subtitle(
    state: State<'_, Desktop>,
    session_id: String,
    name: String,
) -> Result<String, String> {
    let path = source_path(&state, &session_id).await?;
    tauri::async_runtime::spawn_blocking(move || read_sidecar(&path, &name))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

async fn extract_vtt(ffmpeg: &Path, source: &Path, order: usize) -> Result<String> {
    let mut cmd = Command::new(ffmpeg);
    cmd.args([
        "-nostdin",
        "-v",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-i",
    ])
    .arg(source)
    .args([
        "-map",
        &format!("0:s:{order}"),
        "-c:s",
        "webvtt",
        "-f",
        "webvtt",
        "pipe:1",
    ]);
    Ok(String::from_utf8(tool_output(cmd, MAX_BYTES).await?)?)
}

#[tauri::command]
pub async fn read_embedded_subtitle(
    state: State<'_, Desktop>,
    session_id: String,
    order: usize,
) -> Result<String, String> {
    let path = source_path(&state, &session_id).await?;
    extract_vtt(&Tools::resolve().ffmpeg, &path, order)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_subtitles_for_the_open_movie_only() {
        let movie = Path::new("A short film.mkv");
        for name in ["A short film.srt", "A short film.en-US.sdh.VTT"] {
            assert!(matches_movie(movie, name));
        }
        for name in [
            "A short film notes.srt",
            "A short film.txt",
            "../A short film.srt",
            "A short film.en/else.vtt",
            "A short film.\\remote.srt",
        ] {
            assert!(!matches_movie(movie, name));
        }
    }

    #[test]
    fn lists_and_reads_only_neighboring_sidecars() {
        let dir = std::env::temp_dir().join(format!("hlp-neighbor-tracks-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let movie = dir.join("film.mkv");
        for name in ["film.srt", "film.en.VTT", "notes.srt", "film.mp4"] {
            std::fs::write(dir.join(name), "Station").unwrap();
        }
        assert_eq!(sidecars(&movie), ["film.en.VTT", "film.srt"]);
        assert_eq!(read_sidecar(&movie, "film.srt").unwrap(), "Station");
        assert!(read_sidecar(&movie, "../notes.srt").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reads_a_utf16_sidecar() {
        let path = std::env::temp_dir().join(format!("hlp-track-{}.srt", std::process::id()));
        let body = "1\r\n00:00:01,000 --> 00:00:02,000\r\n日本語\r\n";
        let bytes = [0xff, 0xfe]
            .into_iter()
            .chain(body.encode_utf16().flat_map(u16::to_le_bytes))
            .collect::<Vec<_>>();
        std::fs::write(&path, bytes).unwrap();
        let file = read_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(file.text, body);
    }

    #[tokio::test]
    #[ignore = "requires installed FFmpeg and FFprobe"]
    async fn reads_real_embedded_tracks() {
        let dir = std::env::temp_dir().join(format!("hlp-embedded-tracks-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("bilingual.mkv");
        let mut mux = Command::new(Tools::resolve().ffmpeg);
        mux.args(["-y", "-nostdin", "-v", "error"]);
        for (lang, text) in [("eng", "Station"), ("jpn", "駅です")] {
            let path = dir.join(format!("{lang}.srt"));
            std::fs::write(&path, format!("1\n00:00:01,000 --> 00:00:03,000\n{text}\n")).unwrap();
            mux.arg("-i").arg(path);
        }
        let status = mux
            .args([
                "-map",
                "0:s",
                "-map",
                "1:s",
                "-c",
                "copy",
                "-metadata:s:s:0",
                "language=eng",
                "-metadata:s:s:1",
                "language=jpn",
                "-disposition:s:0",
                "default",
                "-disposition:s:1",
                "forced",
            ])
            .arg(&source)
            .status()
            .await
            .unwrap();
        assert!(status.success());
        let tools = Tools::resolve();
        let tracks = embedded_tracks(&tools.ffprobe, &source).await.unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].language.as_deref(), Some("eng"));
        assert_eq!(tracks[1].language.as_deref(), Some("jpn"));
        assert!(tracks[0].default && !tracks[0].forced);
        assert!(tracks[1].forced && !tracks[1].default);
        for (order, expected) in [(0, "Station"), (1, "駅です")] {
            let text = extract_vtt(&tools.ffmpeg, &source, order).await.unwrap();
            assert!(text.starts_with("WEBVTT"));
            assert!(text.contains(expected));
        }
        assert!(extract_vtt(&tools.ffmpeg, &source, 8).await.is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
