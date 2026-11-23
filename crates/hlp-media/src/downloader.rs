use crate::{process, Tools};
use anyhow::{bail, ensure, Context, Result};
use hlp_core::types::{DownloadProgress, UrlInfo};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

const FORMAT: &str = "bv*[vcodec^=avc1][height<=1080]+ba[acodec^=mp4a]/b[ext=mp4][height<=1080]/bv*[height<=1080]+ba/b";
const TEMPLATE: &str = "download:HLPPROG %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s %(progress.eta)s %(info.format_id)s";

pub struct Downloader {
    tools: Tools,
}
impl Downloader {
    pub fn new(tools: Tools) -> Self {
        Self { tools }
    }
    fn command(&self, url: &str) -> Result<Command> {
        let parsed = reqwest::Url::parse(url).context("Enter an HTTP or HTTPS video URL")?;
        ensure!(
            matches!(parsed.scheme(), "https" | "http") && parsed.host().is_some(),
            "Enter an HTTP or HTTPS video URL"
        );
        let mut cmd = Command::from(process::command(&self.tools.yt_dlp));
        cmd.kill_on_drop(true)
            .args([
                "--ignore-config",
                "--no-playlist",
                "--no-colors",
                "--js-runtimes",
            ])
            .arg(format!("deno:{}", self.tools.deno.display()))
            .arg("--ffmpeg-location")
            .arg(
                self.tools
                    .ffmpeg
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(&self.tools.ffmpeg),
            );
        Ok(cmd)
    }
    pub async fn probe(&self, url: &str, cancel: CancellationToken) -> Result<UrlInfo> {
        let mut cmd = self.command(url)?;
        cmd.args(["--dump-single-json", "--no-warnings", "--", url]);
        let mut json = None;
        invoke(&mut cmd, cancel, |line| {
            if line.starts_with('{') {
                json = Some(serde_json::from_str::<serde_json::Value>(line)?);
            }
            Ok(())
        })
        .await?;
        let v = json.context("yt-dlp returned no video details")?;
        ensure!(
            !matches!(v["_type"].as_str(), Some("playlist" | "multi_video"))
                && !v["entries"].is_array(),
            "Playlists are not supported"
        );
        let live = v["is_live"].as_bool() == Some(true)
            || matches!(v["live_status"].as_str(), Some("is_live" | "is_upcoming"));
        ensure!(!live, "Live streams are not supported");
        Ok(UrlInfo {
            id: v["id"].as_str().context("Missing video ID")?.into(),
            title: v["title"].as_str().context("Missing video title")?.into(),
            duration: v["duration"].as_f64(),
            extractor: v["extractor"].as_str().unwrap_or_default().into(),
            thumbnail: v["thumbnail"].as_str().map(str::to_owned),
            is_live: false,
        })
    }

    pub async fn download(
        &self,
        url: &str,
        info: &UrlInfo,
        dir: &Path,
        progress: impl Fn(DownloadProgress) + Send + Sync,
        cancel: CancellationToken,
    ) -> Result<PathBuf> {
        ensure!(!info.is_live, "Live streams are not supported");
        ensure!(!cancel.is_cancelled(), "Cancelled");
        let mut cmd = self.command(url)?;
        fs::create_dir_all(dir)?;
        let dir = dir.canonicalize()?;
        let staging = Staging::new(&dir)?;
        // yt-dlp publishes the completed file to home after merging. All intermediate
        // formats and fragment files belong to this download's private temp directory.
        cmd.args([
            "--newline",
            "--progress",
            "--no-simulate",
            "--no-overwrites",
            "--match-filter",
            "!is_live",
            "-f",
            FORMAT,
            "--merge-output-format",
            "mp4",
            "-o",
            "%(title).80B [%(id)s].%(ext)s",
            "-P",
        ])
        .arg(format!("home:{}", dir.display()))
        .arg("-P")
        .arg(format!("temp:{}", staging.0.display()))
        .args([
            "--progress-template",
            TEMPLATE,
            "--print",
            "before_dl:HLPPARTS %(requested_formats.:.format_id)j",
            "--print",
            "after_move:filepath",
            "--",
            url,
        ]);
        let mut result = None;
        let mut state = Progress::default();
        invoke(&mut cmd, cancel, |line| {
            if let Some(update) = state.line(line) {
                progress(update);
            } else if let Ok(file) = Path::new(line).canonicalize() {
                if file.is_file() && file.parent() == Some(dir.as_path()) {
                    result = Some(file);
                }
            }
            Ok(())
        })
        .await?;
        result.context("yt-dlp finished without a completed file")
    }
}

struct Staging(PathBuf);
impl Staging {
    fn new(dir: &Path) -> Result<Self> {
        loop {
            let path = dir.join(format!(".hlp-download-{:032x}", rand::random::<u128>()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            tracing::warn!(%error,"Could not clean up download parts");
        }
    }
}

#[derive(Default)]
struct Progress {
    parts: u32,
    part: u32,
    last_format: Option<String>,
    last_bytes: u64,
}
impl Progress {
    fn line(&mut self, line: &str) -> Option<DownloadProgress> {
        if let Some(formats) = line.strip_prefix("HLPPARTS ") {
            self.parts =
                serde_json::from_str::<Vec<String>>(formats).map_or(1, |v| v.len().max(1) as u32);
            return None;
        }
        let mut fields = line.strip_prefix("HLPPROG ")?.split_whitespace();
        let downloaded = fields.next()?.parse().ok()?;
        let total = fields.next()?.parse().ok();
        let estimate = fields.next()?.parse().ok();
        let eta = fields.next()?.parse().ok();
        let format = fields.next().filter(|s| *s != "NA");
        if self.part == 0 {
            self.part = 1;
        } else if format
            .zip(self.last_format.as_deref())
            .is_some_and(|(a, b)| a != b)
            || (format.is_none() && downloaded < self.last_bytes)
        {
            self.part += 1;
        }
        self.last_format = format.map(str::to_owned);
        self.last_bytes = downloaded;
        Some(DownloadProgress {
            downloaded,
            total: total.or(estimate),
            eta,
            part: self.part,
            parts: self.parts.max(self.part),
        })
    }
}

// A cancelled command or dropped future releases the whole subprocess tree. This
// includes the ffmpeg merger and Deno runtime started by yt-dlp.
struct ChildTree(Option<u32>);
impl Drop for ChildTree {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            process::kill_tree(pid);
        }
    }
}

async fn invoke(
    cmd: &mut Command,
    cancel: CancellationToken,
    mut line: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    ensure!(!cancel.is_cancelled(), "Cancelled");
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Start yt-dlp")?;
    let mut tree = ChildTree(child.id());
    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();
    let drain = async {
        let output = async {
            while let Some(row) = stdout.next_line().await? {
                line(&row)?;
            }
            Ok::<_, anyhow::Error>(())
        };
        let errors = async {
            let mut last = String::new();
            while let Some(row) = stderr.next_line().await? {
                if !row.trim().is_empty() {
                    last = row;
                }
            }
            Ok::<_, anyhow::Error>(last)
        };
        let ((), last) = tokio::try_join!(output, errors)?;
        let status = child.wait().await?;
        Ok::<_, anyhow::Error>((status, last))
    };
    let result = tokio::select! {biased; _=cancel.cancelled()=>Err(anyhow::anyhow!("Cancelled")), r=drain=>r };
    match result {
        Ok((status, last)) => {
            tree.0 = None;
            if !status.success() {
                bail!("yt-dlp ({status}): {last}");
            }
            Ok(())
        }
        Err(error) => {
            drop(tree);
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn part_and_estimate() {
        let mut p = Progress::default();
        p.line(r#"HLPPARTS ["avc","aac"]"#);
        let a = p.line("HLPPROG 400 NA 700 NA avc").unwrap();
        assert_eq!((a.total, a.eta, a.part, a.parts), (Some(700), None, 1, 2));
        let b = p.line("HLPPROG 790 1000 NA 1 aac").unwrap();
        assert_eq!((b.part, b.total, b.eta), (2, Some(1000), Some(1)));
    }
}
