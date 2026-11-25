//! Atomic per-track state shared by the CLI and desktop player.
use anyhow::{Context, Result};
use hlp_core::{events::LanguageEvent, types::*};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn data_root() -> PathBuf {
    std::env::var_os("HLP_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_default()
                .join("com.henlocal.player")
        })
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn read<T: DeserializeOwned>(path: &Path) -> Option<T> {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!(file=%path.display(),%e,"Ignoring invalid cache JSON");
                None
            }
        },
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(file=%path.display(),%e,"Cannot read cache")
            };
            None
        }
    }
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    atomic_write(path, &serde_json::to_vec(value)?)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.flush()?;
    tmp.persist(path)
        .with_context(|| format!("Write {}", path.display()))?;
    Ok(())
}

/// Clear only the generated text and the records that request recognition.
/// Audio, prepared video, authored tracks and exported subtitle files are separate.
pub(crate) fn remove_generated_subtitles(track: &Path) -> Result<()> {
    fn remove_if_present(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).with_context(|| format!("Remove {}", path.display())),
        }
    }
    for entry in fs::read_dir(track)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(
            name.as_ref(),
            "generate.requested" | "segments.json" | "transcript.json"
        ) || (name.starts_with("transcript.") && name.ends_with(".json"))
        {
            remove_if_present(&entry.path())?;
        }
    }
    let translations = track.join("translations");
    if translations.is_symlink() {
        // Unlink a redirected cache without touching the referenced directory.
        remove_if_present(&translations)?;
    } else if translations.try_exists()? {
        for entry in fs::read_dir(translations)? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                remove_if_present(&path)?;
            }
        }
    }
    Ok(())
}
pub fn hash(text: &str) -> String {
    hex::encode(&Sha256::digest(text.as_bytes())[..8])
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
    pub plan: PlaybackPlan,
    pub playable_path: Option<PathBuf>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Prepared {
    pub caps: Caps,
    pub plan: PlaybackPlan,
}
pub(crate) fn prepared_playback(track: &Path, file: &Path, caps: &Caps) -> Option<Playback> {
    let saved: Prepared = read(&track.join("playback.json"))?;
    (saved.caps == *caps && saved.plan != PlaybackPlan::Direct && file.is_file()).then(|| {
        Playback {
            plan: saved.plan,
            playable_path: Some(file.to_owned()),
        }
    })
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub fingerprint: String,
    pub source_path: PathBuf,
    pub source_url: Option<String>,
    pub title: String,
    pub duration_ms: Ms,
    pub probe: MediaInfo,
    pub audio_track_order: usize,
    pub playback: Playback,
    pub language: LanguageEvent,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Audio {
    pub sample_rate: u32,
    pub total_samples: u64,
    pub complete: bool,
    pub dialogue_focus: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segments {
    pub vad_params_hash: String,
    pub complete: bool,
    pub utterances: Vec<Utterance>,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    #[serde(default)]
    pub source_mode: String,
    #[serde(default)]
    pub generation: u64,
    pub asr_model: String,
    pub vad_params_hash: String,
    pub items: BTreeMap<String, Utterance>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    pub source_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,
    /// Hash of the effective source language and the exact background sent to Hy-MT2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_hash: Option<String>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aligned_phrases: Vec<hlp_core::cues::AlignedPhrase>,
    #[serde(default)]
    pub failed: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Translations {
    pub translator_id: String,
    pub prompt_version: u32,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub context: bool,
    pub items: BTreeMap<String, Translation>,
    /// Still-readable subtitles from the other mode while the requested mode catches up.
    #[serde(skip)]
    pub fallback: BTreeMap<String, Translation>,
}
impl Translations {
    pub fn path(track: &Path, target: &str, context: bool) -> PathBuf {
        let filename = if context {
            format!("{target}.context.json")
        } else {
            format!("{target}.json")
        };
        track.join("translations").join(filename)
    }

    pub fn load(track: &Path, target: &str, context: bool, generation: u64) -> Self {
        let version = hlp_core::prompts::PROMPT_VERSION;
        let read_for_mode = |mode| {
            read::<Self>(&Self::path(track, target, mode)).filter(|t| {
                t.translator_id == "hy-mt2-1.8b-q4km"
                    && t.prompt_version == version
                    && t.generation == generation
            })
        };
        let mut active = read_for_mode(context).unwrap_or_else(|| Self {
            translator_id: "hy-mt2-1.8b-q4km".into(),
            prompt_version: version,
            generation,
            context,
            items: BTreeMap::new(),
            fallback: BTreeMap::new(),
        });
        if active.context != context {
            // Early players used the plain filename for both modes. Keep those
            // captions readable until each has a replacement for this mode.
            for entry in active.items.values_mut() {
                entry.context_hash = Some(String::new());
            }
            active.context = context;
        }
        active.fallback = read_for_mode(!context).map(|t| t.items).unwrap_or_default();
        active
    }
}
pub fn list_recent(root: &Path) -> Vec<RecentItem> {
    read(&root.join("recent.json")).unwrap_or_default()
}
pub fn record_recent(root: &Path, project: &Project, progress: f32, opened_at: u64) -> Result<()> {
    use fs2::FileExt;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join(".recent.lock"))?;
    lock.lock_exclusive()?;
    let mut items = list_recent(root);
    items.retain(|v| v.fingerprint != project.fingerprint);
    items.insert(
        0,
        RecentItem {
            fingerprint: project.fingerprint.clone(),
            path: project.source_path.to_string_lossy().into_owned(),
            title: project.title.clone(),
            duration_ms: project.duration_ms,
            last_opened: opened_at,
            progress,
        },
    );
    items.sort_by_key(|item| std::cmp::Reverse(item.last_opened));
    items.truncate(20);
    write_json(&root.join("recent.json"), &items)
}
pub fn cache_size(dir: &Path) -> u64 {
    let Ok(entries) = dir.read_dir() else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(t) if t.is_dir() => cache_size(&entry.path()),
            Ok(t) if t.is_file() => entry.metadata().map_or(0, |m| m.len()),
            _ => 0,
        })
        .sum()
}
pub fn lock_project(root: &Path, fp: &str) -> Result<fs::File> {
    let dir = root.join("locks");
    fs::create_dir_all(&dir)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(fp))?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .context("This video is already open in another player session")?;
    Ok(lock)
}
pub fn evict(root: &Path, current: &str, limit: u64) -> Result<()> {
    let media = root.join("media");
    let mut size = cache_size(&media);
    if size <= limit {
        return Ok(());
    }
    let mut dirs = Vec::new();
    for entry in media.read_dir()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name != current && entry.file_type()?.is_dir() {
            let date =
                read::<Project>(&entry.path().join("project.json")).map_or(0, |p| p.updated_at);
            dirs.push((date, name, entry.path()));
        }
    }
    dirs.sort();
    for only_playable in [true, false] {
        for (_, name, path) in &dirs {
            let Ok(_guard) = lock_project(root, name) else {
                continue;
            };
            // A concurrent opener can evict an inactive project before we obtain its lock.
            if !path.is_dir() {
                continue;
            }
            if only_playable {
                for entry in path.read_dir()?.flatten() {
                    if entry.file_name().to_string_lossy().starts_with("playable-")
                        && entry.file_type()?.is_file()
                    {
                        let len = entry.metadata()?.len();
                        fs::remove_file(entry.path())?;
                        size = size.saturating_sub(len)
                    }
                }
            } else {
                let len = cache_size(path);
                fs::remove_dir_all(path)?;
                size = size.saturating_sub(len)
            }
            if size <= limit {
                return Ok(());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_prepared_tracks_and_respects_changed_capabilities_or_eviction() {
        let temp = tempfile::tempdir().unwrap();
        let track = temp.path().join("track-1");
        let file = temp.path().join("playable-1.mp4");
        let caps = Caps::from([("video/webm".into(), "probably".into())]);
        write_json(
            &track.join("playback.json"),
            &Prepared {
                caps: caps.clone(),
                plan: PlaybackPlan::Transcode,
            },
        )
        .unwrap();
        atomic_write(&file, b"prepared").unwrap();
        assert_eq!(
            prepared_playback(&track, &file, &caps).unwrap().plan,
            PlaybackPlan::Transcode
        );
        assert!(prepared_playback(&track, &file, &Caps::new()).is_none());
        fs::remove_file(&file).unwrap();
        assert!(prepared_playback(&track, &file, &caps).is_none());
    }
    #[test]
    fn eviction_removes_playable_files_first_and_respects_open_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        for project in ["open", "locked", "inactive"] {
            let dir = root.join("media").join(project);
            atomic_write(&dir.join("playable-0.mp4"), &[0; 100]).unwrap();
            atomic_write(&dir.join("track-0/transcript.json"), b"{}").unwrap();
        }
        let _other_session = lock_project(root, "locked").unwrap();
        assert!(lock_project(root, "locked").is_err());
        evict(root, "open", 206).unwrap();
        assert!(!root.join("media/inactive/playable-0.mp4").exists());
        assert!(root
            .join("media/inactive/track-0/transcript.json")
            .is_file());
        evict(root, "open", 0).unwrap();
        assert!(!root.join("media/inactive").exists());
        for kept in ["open", "locked"] {
            assert!(root
                .join("media")
                .join(kept)
                .join("playable-0.mp4")
                .is_file())
        }
    }
}
