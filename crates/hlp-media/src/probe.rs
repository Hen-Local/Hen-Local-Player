use crate::{process, Tools};
use anyhow::{bail, Context, Result};
use hlp_core::types::{AudioStream, MediaInfo, VideoStream};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::Read, path::Path};

#[derive(Default, Deserialize)]
#[serde(default)]
struct Stream {
    index: usize,
    codec_type: String,
    codec_name: String,
    codec_tag_string: String,
    profile: String,
    pix_fmt: String,
    width: u32,
    height: u32,
    field_order: String,
    channels: u32,
    channel_layout: String,
    sample_rate: String,
    duration: String,
    disposition: HashMap<String, i32>,
    tags: HashMap<String, String>,
}

#[derive(Deserialize)]
struct Report {
    streams: Vec<Stream>,
    format: Format,
}

#[derive(Deserialize)]
struct Format {
    format_name: String,
    #[serde(default)]
    duration: String,
}

fn milliseconds(seconds: &str) -> u64 {
    seconds
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
        .map_or(0, |n| (1000.0 * n).round() as u64)
}

fn parse(bytes: &[u8], container_type: Option<&str>) -> Result<MediaInfo> {
    let report: Report = serde_json::from_slice(bytes).context("Invalid ffprobe JSON")?;
    let mut info = MediaInfo {
        container: container_type
            .unwrap_or(&report.format.format_name)
            .to_owned(),
        duration_ms: milliseconds(&report.format.duration),
        video: None,
        audio: Vec::new(),
        has_subtitle_streams: false,
    };
    let mut longest = 0;
    for stream in report.streams {
        longest = longest.max(milliseconds(&stream.duration));
        match stream.codec_type.as_str() {
            "video"
                if info.video.is_none() && stream.disposition.get("attached_pic") != Some(&1) =>
            {
                info.video = Some(VideoStream {
                    codec_name: stream.codec_name,
                    codec_tag_string: stream.codec_tag_string,
                    profile: stream.profile,
                    pix_fmt: stream.pix_fmt,
                    width: stream.width,
                    height: stream.height,
                    field_order: stream.field_order,
                });
            }
            "audio" => info.audio.push(AudioStream {
                index: stream.index,
                order: info.audio.len(),
                codec_name: stream.codec_name,
                channels: stream.channels,
                channel_layout: stream.channel_layout,
                sample_rate: stream.sample_rate.parse().unwrap_or_default(),
                language_tag: stream.tags.get("language").cloned(),
                default: stream.disposition.get("default") == Some(&1),
            }),
            "subtitle" => info.has_subtitle_streams = true,
            _ => {}
        }
    }
    if info.audio.is_empty() && info.video.is_none() {
        bail!("This file has no playable video or audio");
    }
    if info.duration_ms == 0 {
        info.duration_ms = longest;
    }
    Ok(info)
}

/// The Matroska and WebM demuxers have one shared ffprobe name. Read the short
/// EBML header's DocType field to distinguish these without trusting a suffix.
fn ebml_type(path: &Path) -> Option<&'static str> {
    let mut header = Vec::new();
    File::open(path)
        .ok()?
        .take(4096)
        .read_to_end(&mut header)
        .ok()?;
    if !header.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        return None;
    }
    if header.windows(7).any(|w| w == b"\x42\x82\x84webm") {
        Some("webm")
    } else {
        Some("matroska")
    }
}

pub fn probe(tools: &Tools, path: &Path) -> Result<MediaInfo> {
    let input = path
        .canonicalize()
        .with_context(|| format!("Open {}", path.display()))?;
    let output = process::command(&tools.ffprobe)
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            "-i",
        ])
        .arg(&input)
        .output()
        .with_context(|| format!("Start ffprobe at {}", tools.ffprobe.display()))?;
    if !output.status.success() {
        bail!(
            "ffprobe: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    parse(&output.stdout, ebml_type(&input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decide, default_caps};
    use hlp_core::types::PlaybackPlan::*;
    #[test]
    fn codec_matrix_from_ffprobe() {
        for (json, container, expected) in [
            (
                include_bytes!("../tests/probes/ja.json").as_slice(),
                None,
                Direct,
            ),
            (
                include_bytes!("../tests/probes/fr.json").as_slice(),
                Some("matroska"),
                Remux,
            ),
            (
                include_bytes!("../tests/probes/de.json").as_slice(),
                Some("webm"),
                Direct,
            ),
            (
                include_bytes!("../tests/probes/es.json").as_slice(),
                None,
                Transcode,
            ),
            (
                include_bytes!("../tests/probes/hevc.json").as_slice(),
                Some("matroska"),
                Remux,
            ),
            (
                include_bytes!("../tests/probes/prores.json").as_slice(),
                None,
                Transcode,
            ),
            (
                include_bytes!("../tests/probes/flac.json").as_slice(),
                None,
                Direct,
            ),
        ] {
            let info = parse(json, container).unwrap();
            assert_eq!(
                decide(&info, &default_caps("wkwebview"), 0),
                expected,
                "{:?}",
                info
            );
        }
    }
    #[test]
    fn silent_movies_use_the_same_video_capability_checks() {
        for (container, codec, expected) in [
            ("mov", "h264", Direct),
            ("matroska", "h264", Remux),
            ("avi", "mpeg4", Transcode),
            ("webm", "vp9", Direct),
        ] {
            let report = serde_json::json!({
                "format": { "format_name": container, "duration": "8.0" },
                "streams": [{"codec_type": "video", "codec_name": codec, "width": 640, "height": 360}]
            });
            let media = parse(&serde_json::to_vec(&report).unwrap(), None).unwrap();
            assert!(media.audio.is_empty());
            assert_eq!(decide(&media, &default_caps("wkwebview"), 0), expected);
        }
    }

    #[test]
    fn rejects_files_without_media() {
        assert_eq!(
            parse(br#"{"streams":[],"format":{"format_name":"mov"}}"#, None)
                .unwrap_err()
                .to_string(),
            "This file has no playable video or audio"
        );
    }
}
