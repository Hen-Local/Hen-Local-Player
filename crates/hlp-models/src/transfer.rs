// Range resume, retry and digest checks adapted from Hen-Local-Translator/hen-local-init/src/{gguf,main}.rs.
use crate::{file_len, manifest::Artifact};
use anyhow::{bail, ensure, Context, Result};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("Hen-Local-Player/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(75))
        .build()?)
}
pub(super) fn mainland_region(locale: &str) -> bool {
    locale
        .split(['-', '_', '.', '@'])
        .skip(1)
        .take_while(|part| part.len() > 1)
        .any(|part| part.eq_ignore_ascii_case("CN"))
}

// Give the regional source a head start. Probe the pinned file itself so a
// reachable home page or an error page cannot select an unavailable model.
pub(super) async fn prefer_mirror(
    client: &Client,
    artifact: &Artifact,
    mainland: bool,
    cancel: &CancellationToken,
) -> Result<bool> {
    let Some(mirror) = artifact.mirror.as_deref() else {
        return Ok(false);
    };
    let (first, second) = if mainland {
        (mirror, artifact.url.as_str())
    } else {
        (artifact.url.as_str(), mirror)
    };
    let probe = |url: String| async {
        let response = client
            .get(url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .timeout(Duration::from_secs(4))
            .send()
            .await
            .ok()?;
        if !matches!(
            response.status(),
            StatusCode::OK | StatusCode::PARTIAL_CONTENT
        ) || response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("text/html"))
        {
            return None;
        }
        response
            .bytes_stream()
            .next()
            .await?
            .ok()
            .filter(|bytes| !bytes.is_empty())
    };
    let preferred = probe(first.to_owned());
    tokio::pin!(preferred);
    tokio::select! {
        _ = cancel.cancelled() => bail!("Setup cancelled"),
        result = &mut preferred => return Ok(if result.is_some() { mainland } else { !mainland }),
        _ = tokio::time::sleep(Duration::from_millis(650)) => {}
    }
    tokio::select! {
        _ = cancel.cancelled() => bail!("Setup cancelled"),
        result = &mut preferred => Ok(if result.is_some() { mainland } else { !mainland }),
        result = probe(second.to_owned()) => Ok(if result.is_some() { !mainland } else { mainland }),
    }
}
pub fn part_path(stage: &Path, artifact: &Artifact) -> PathBuf {
    stage.join(format!("{}.part", artifact.sha256))
}
pub async fn verify(path: &Path, expected: &str, cancel: &CancellationToken) -> Result<bool> {
    let (path, expected, cancel) = (path.to_owned(), expected.to_owned(), cancel.clone());
    tokio::task::spawn_blocking(move || {
        let mut file = File::open(&path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 256 * 1024];
        loop {
            ensure!(!cancel.is_cancelled(), "Setup cancelled");
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        Ok(hex::encode(hasher.finalize()) == expected)
    })
    .await?
}
async fn fetch(
    client: &Client,
    url: &str,
    path: &Path,
    expected: u64,
    progress: &impl Fn(u64),
    cancel: &CancellationToken,
) -> Result<()> {
    let mut offset = file_len(path);
    if offset > expected {
        tokio::fs::remove_file(path).await?;
        offset = 0;
    }
    if offset == expected {
        return Ok(());
    }
    let mut req = client.get(url);
    if offset > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={offset}-"));
    }
    let response =
        tokio::select! { _=cancel.cancelled()=>bail!("Setup cancelled"),res=req.send()=>res? }
            .error_for_status()?;
    let partial = response.status() == StatusCode::PARTIAL_CONTENT;
    if partial {
        let header = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();
        let (range, total) = header
            .strip_prefix("bytes ")
            .and_then(|h| h.split_once('/'))
            .context("Missing byte range in resumed response")?;
        let (start, end) = range.split_once('-').context("Invalid content range")?;
        ensure!(
            start.parse::<u64>()? == offset
                && end.parse::<u64>()? < expected
                && total.parse::<u64>()? == expected,
            "Server returned a different byte range"
        );
    } else {
        ensure!(
            response.status() == StatusCode::OK,
            "Unexpected HTTP status {}",
            response.status()
        );
        offset = 0;
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(partial)
        .truncate(!partial)
        .open(path)
        .await?;
    let mut stream = response.bytes_stream();
    let mut report = Instant::now();
    progress(offset);
    loop {
        let chunk = tokio::select! {_=cancel.cancelled()=>{file.flush().await?;bail!("Setup cancelled")},v=stream.next()=>v};
        let Some(chunk) = chunk else { break };
        let bytes = chunk?;
        ensure!(
            bytes.len() as u64 <= expected - offset,
            "Remote file exceeds its pinned length"
        );
        file.write_all(&bytes).await?;
        offset += bytes.len() as u64;
        if report.elapsed() >= Duration::from_millis(150) {
            progress(offset);
            report = Instant::now();
        }
    }
    file.flush().await?;
    ensure!(
        offset == expected,
        "Short download: {offset} of {expected} bytes"
    );
    progress(offset);
    Ok(())
}

pub async fn obtain(
    client: &Client,
    artifact: &Artifact,
    stage: &Path,
    progress: impl Fn(u64),
    verifying: impl Fn(),
    cancel: &CancellationToken,
) -> Result<PathBuf> {
    let path = part_path(stage, artifact);
    let mut last = None;
    for attempt in 0..4 {
        ensure!(!cancel.is_cancelled(), "Setup cancelled");
        let url = if attempt % 2 == 1 {
            artifact.mirror.as_deref().unwrap_or(&artifact.url)
        } else {
            &artifact.url
        };
        match fetch(client, url, &path, artifact.bytes, &progress, cancel).await {
            Ok(()) => {
                verifying();
                if verify(&path, &artifact.sha256, cancel).await? {
                    return Ok(path);
                }
                tokio::fs::remove_file(&path).await?;
                last = Some(anyhow::anyhow!("SHA-256 mismatch for {}", artifact.url));
            }
            Err(err) => last = Some(err),
        }
        if attempt < 3 {
            tokio::select! {_=cancel.cancelled()=>bail!("Setup cancelled"),_=tokio::time::sleep(Duration::from_millis(300<<attempt))=>{}}
        }
    }
    Err(last.expect("four attempts")).with_context(|| format!("Could not install {}", artifact.url))
}
