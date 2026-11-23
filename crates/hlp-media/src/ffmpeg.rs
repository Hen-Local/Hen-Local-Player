use crate::{
    capabilities::{copy_audio, interlaced},
    process, Tools,
};
use anyhow::{bail, ensure, Context, Result};
use hlp_core::types::*;
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};
use tokio_util::sync::CancellationToken;

fn ffmpeg(tools: &Tools) -> Command {
    let mut cmd = process::command(&tools.ffmpeg);
    cmd.args([
        "-nostdin",
        "-hide_banner",
        "-v",
        "error",
        "-nostats",
        "-progress",
        "pipe:2",
        "-y",
    ]);
    cmd
}

fn front_center(layout: &str) -> bool {
    matches!(
        layout,
        "mono"
            | "3.0"
            | "3.1"
            | "4.0"
            | "4.1"
            | "5.0"
            | "5.0(side)"
            | "5.1"
            | "5.1(side)"
            | "6.0"
            | "6.1"
            | "6.1(back)"
            | "7.0"
            | "7.1"
            | "7.1(wide)"
            | "7.1(wide-side)"
            | "octagonal"
            | "hexagonal"
    ) || layout
        .split(|c: char| !c.is_alphanumeric())
        .any(|ch| ch == "FC")
}

/// Write and publish frames in sample order. A short final frame carries the exact remainder.
/// The return value is the number of 16 kHz samples committed to the PCM file.
pub fn extract_audio(
    tools: &Tools,
    input: &Path,
    track: &AudioStream,
    dialogue_focus: bool,
    pcm: &Path,
    mut on_frame: impl FnMut(&[f32], Ms) -> Result<()>,
    cancel: CancellationToken,
) -> Result<u64> {
    let mut cmd = ffmpeg(tools);
    cmd.arg("-i")
        .arg(input.canonicalize()?)
        .arg("-map")
        .arg(format!("0:a:{}", track.order))
        .args(["-vn", "-sn", "-dn"]);
    // Anchor PCM sample zero to the media timeline. AAC priming and gaps in
    // joined source streams would otherwise cause cumulative subtitle drift.
    let filter = if dialogue_focus && track.channels > 1 && front_center(&track.channel_layout) {
        "pan=mono|c0=FC,aresample=16000:async=1:first_pts=0"
    } else {
        "aresample=16000:async=1:first_pts=0"
    };
    cmd.args(["-af", filter, "-ac", "1"]);
    cmd.args([
        "-ar",
        "16000",
        "-f",
        "s16le",
        "-acodec",
        "pcm_s16le",
        "pipe:1",
    ])
    .stdout(Stdio::piped());
    if let Some(dir) = pcm.parent() {
        fs::create_dir_all(dir)?;
    }
    let (status, (samples, diagnostic)) = process::run(&mut cmd, &cancel, |out, err| {
        thread::scope(|scope| {
            let stderr = scope.spawn(move || process::error_tail(BufReader::new(err)));
            // Drop the read end promptly on callback or disk failure, even while stderr drains.
            let result = (|| -> Result<u64> {
                let mut pipe = BufReader::new(out.expect("piped PCM"));
                let mut file = File::create(pcm)?;
                let mut bytes = [0u8; 1024];
                let mut frame = [0f32; 512];
                let mut total = 0u64;
                loop {
                    let mut n = 0;
                    while n < bytes.len() {
                        let count = pipe.read(&mut bytes[n..])?;
                        if count == 0 {
                            break;
                        }
                        n += count;
                    }
                    if n == 0 {
                        break;
                    }
                    ensure!(n % 2 == 0, "Truncated PCM sample");
                    file.write_all(&bytes[..n])?;
                    for (dst, src) in frame.iter_mut().zip(bytes[..n].chunks_exact(2)) {
                        *dst = i16::from_le_bytes([src[0], src[1]]) as f32 / 32768.;
                    }
                    total += (n / 2) as u64;
                    on_frame(&frame[..n / 2], total / 16)?;
                }
                file.flush()?;
                Ok(total)
            })();
            let diagnostic = stderr
                .join()
                .map_err(|_| anyhow::anyhow!("ffmpeg diagnostic reader stopped"))??;
            Ok((result?, diagnostic))
        })
    })?;
    ensure!(
        status.success(),
        "ffmpeg audio extraction failed ({status}): {diagnostic}"
    );
    Ok(samples)
}

fn progress_fraction(line: &str, duration_ms: Ms) -> Option<f32> {
    if line == "progress=end" {
        return Some(1.0);
    }
    let micros = line
        .strip_prefix("out_time_us=")?
        .trim()
        .parse::<i64>()
        .ok()?
        .max(0) as f64;
    (duration_ms != 0).then(|| (micros / (duration_ms as f64 * 1000.)).clamp(0., 1.) as f32)
}

fn cached(output: &Path, plan: &PlaybackPlan, order: usize) -> bool {
    let Some(dir) = output.parent() else {
        return false;
    };
    if !output.is_file() {
        return false;
    }
    let Some(project) = fs::read(dir.join("project.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
    else {
        return false;
    };
    let playback = &project["playback"];
    project["audioTrackOrder"].as_u64() == Some(order as u64)
        && serde_json::from_value::<PlaybackPlan>(playback["plan"].clone())
            .as_ref()
            .ok()
            == Some(plan)
        && playback["playablePath"].as_str().is_some_and(|p| {
            let path = Path::new(p);
            path == output || (!path.is_absolute() && dir.join(path) == output)
        })
}

#[cfg(windows)]
fn encoders(tools: &Tools) -> Result<Vec<&'static str>> {
    let output = process::command(&tools.ffmpeg)
        .args(["-nostdin", "-hide_banner", "-encoders"])
        .output()?;
    ensure!(
        output.status.success(),
        "Could not list Windows FFmpeg encoders"
    );
    let listing = String::from_utf8_lossy(&output.stdout);
    // The managed LGPL build includes OpenH264, but omits GPL-only libx264.
    // Retain a CPU fallback when hardware and Media Foundation cannot initialize.
    let available: Vec<_> = [
        "h264_nvenc",
        "h264_qsv",
        "h264_amf",
        "h264_mf",
        "libx264",
        "libopenh264",
    ]
    .into_iter()
    .filter(|name| {
        listing
            .lines()
            .any(|line| line.split_whitespace().nth(1) == Some(*name))
    })
    .collect();
    ensure!(
        !available.is_empty(),
        "No H.264 encoder available in FFmpeg"
    );
    Ok(available)
}
#[cfg(not(windows))]
fn encoders(_tools: &Tools) -> Result<Vec<&'static str>> {
    Ok(vec![if cfg!(target_os = "macos") {
        "h264_videotoolbox"
    } else {
        "libx264"
    }])
}

fn bitrate(height: u32) -> &'static str {
    match height {
        0..=480 => "2500k",
        481..=720 => "5M",
        721..=1080 => "8M",
        _ => "20M",
    }
}

pub fn prepare_playback(
    tools: &Tools,
    input: &Path,
    info: &MediaInfo,
    caps: &Caps,
    track: usize,
    plan: &PlaybackPlan,
    output: &Path,
    mut progress: impl FnMut(f32),
    cancel: CancellationToken,
) -> Result<PathBuf> {
    ensure!(!cancel.is_cancelled(), "Cancelled");
    let input = input.canonicalize()?;
    if *plan == PlaybackPlan::Direct {
        return Ok(input);
    }
    let audio = info
        .audio
        .iter()
        .find(|s| s.order == track)
        .context("Audio track unavailable")?;
    if cached(output, plan, track) {
        progress(1.);
        return Ok(output.to_owned());
    }
    if let Some(dir) = output.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut name = OsString::from(output);
    name.push(".part");
    let part = PathBuf::from(name);
    let codecs = if *plan == PlaybackPlan::Transcode && info.video.is_some() {
        encoders(tools)?
    } else {
        vec!["copy"]
    };
    let mut failures = Vec::new();
    for encoder in codecs {
        let mut cmd = ffmpeg(tools);
        cmd.arg("-i").arg(&input);
        if let Some(video) = &info.video {
            // Capital V ignores album art and thumbnail streams, as does probe().
            cmd.args(["-map", "0:V:0", "-c:v", encoder]);
            if encoder == "copy" {
                if video.codec_name == "hevc" {
                    cmd.args(["-tag:v", "hvc1"]);
                }
            } else {
                if interlaced(&video.field_order) {
                    cmd.args(["-vf", "yadif"]);
                }
                if encoder == "libx264" {
                    cmd.args(["-preset", "veryfast", "-crf", "20"]);
                } else {
                    cmd.args(["-b:v", bitrate(video.height)]);
                }
                cmd.args(["-pix_fmt", "yuv420p"]);
            }
        } else {
            cmd.arg("-vn");
        }
        cmd.arg("-map")
            .arg(format!("0:a:{track}"))
            .args(["-sn", "-dn"]);
        if info.video.is_some() && copy_audio(&audio.codec_name, caps) {
            cmd.args(["-c:a", "copy"]);
        } else {
            cmd.args(["-c:a", "aac", "-b:a", "192k", "-ac", "2"]);
        }
        cmd.args(["-movflags", "+faststart", "-f", "mp4"])
            .arg(&part)
            .stdout(Stdio::null());
        progress(0.);
        let result = process::run(&mut cmd, &cancel, |_, stderr| {
            let mut diagnostic = String::new();
            for line in BufReader::new(stderr).lines() {
                let line = line?;
                if let Some(fraction) = progress_fraction(&line, info.duration_ms) {
                    if fraction < 1. {
                        progress(fraction);
                    }
                } else if !line.contains('=') && !line.trim().is_empty() {
                    diagnostic = line;
                }
            }
            Ok(diagnostic)
        });
        match result {
            Ok((status, _)) if status.success() => {
                let published = fs::rename(&part, output);
                if published.is_err() {
                    let _ = fs::remove_file(&part);
                }
                published.context("Publish prepared media")?;
                progress(1.);
                return Ok(output.to_owned());
            }
            Ok((status, error)) => failures.push(format!("{encoder} ({status}): {error}")),
            Err(error) => {
                let _ = fs::remove_file(&part);
                return Err(error);
            }
        }
        let _ = fs::remove_file(&part);
    }
    bail!("ffmpeg preparation failed: {}", failures.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ffmpeg_progress() {
        assert_eq!(progress_fraction("out_time_us=383000", 1000), Some(0.383));
        assert_eq!(progress_fraction("out_time_us=N/A", 500), None);
        assert_eq!(progress_fraction("progress=end", 0), Some(1.0));
    }
}
