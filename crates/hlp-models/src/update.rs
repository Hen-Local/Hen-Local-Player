//! Explicitly user-requested, checksum-verified yt-dlp updates.
use super::{component, event, executable, install, transfer};
use anyhow::{ensure, Context, Result};
use hlp_core::types::{ComponentState, ComponentStatus};
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use tokio_util::sync::CancellationToken;

pub(crate) fn valid_version(s: &str) -> bool {
    !s.is_empty()
        && s.len() < 60
        && s.starts_with(|c: char| c.is_ascii_digit())
        && s.split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
}
async fn metadata(client: &Client, url: &str, cancel: &CancellationToken) -> Result<Vec<u8>> {
    use futures_util::StreamExt;
    let result = async {
        let mut stream = client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes_stream();
        let mut buf = Vec::new();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            ensure!(
                buf.len() + bytes.len() < 2 * 1024 * 1024,
                "Release metadata exceeds size limit"
            );
            buf.extend_from_slice(&bytes);
        }
        Ok(buf)
    };
    tokio::select! {_=cancel.cancelled()=>anyhow::bail!("Update cancelled"),r=result=>r}
}
pub(crate) async fn update(
    root: &Path,
    progress: &(impl Fn(ComponentStatus) + Sync),
    cancel: &CancellationToken,
) -> Result<()> {
    let mut c = component("yt-dlp")
        .context("URL tools unavailable for this platform")?
        .clone();
    let stage = c.stage(root);
    let _lock = install::lock(&stage, cancel).await?;
    let client = transfer::client()?;
    let release: Release = serde_json::from_slice(
        &metadata(
            &client,
            "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest",
            cancel,
        )
        .await?,
    )?;
    ensure!(
        valid_version(&release.tag_name),
        "Invalid stable yt-dlp release tag"
    );
    let filename = if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp_macos"
    };
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == filename && a.size > 0)
        .context("Standalone binary missing from release")?;
    let base = format!(
        "https://github.com/yt-dlp/yt-dlp/releases/download/{}",
        release.tag_name
    );
    let sums = metadata(&client, &format!("{base}/SHA2-256SUMS"), cancel).await?;
    let sha = std::str::from_utf8(&sums)?
        .lines()
        .find_map(|l| {
            let mut parts = l.split_whitespace();
            let h = parts.next()?;
            (parts.next()?.trim_start_matches('*') == filename
                && h.len() == 64
                && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .then_some(h)
        })
        .context("No SHA-256 for the current platform")?;
    let pin = &mut c.files[0];
    pin.url = format!("{base}/{filename}");
    pin.name = Some(executable("yt-dlp"));
    pin.bytes = asset.size;
    pin.sha256 = sha.to_ascii_lowercase();
    c.directory = std::path::PathBuf::from("tools/yt-dlp").join(release.tag_name);
    install::install_locked(root, &c, &client, progress, cancel).await?;
    progress(event(&c, ComponentState::Ready, c.total()));
    Ok(())
}
