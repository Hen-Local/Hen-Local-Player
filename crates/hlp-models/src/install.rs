// Locking, progress and completion markers adapted from Hen-Local-Translator/hen-local-init/src/main.rs.
use super::{archive, event, executable, file_len, ready, transfer, version_ok};
use crate::manifest::{Check, Component, Format};
use anyhow::{ensure, Context, Result};
use hlp_core::types::{ComponentState, ComponentStatus};
use reqwest::Client;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

pub(crate) async fn lock(stage: &Path, cancel: &CancellationToken) -> Result<File> {
    fs::create_dir_all(stage)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(stage.join("setup.lock"))?;
    loop {
        ensure!(!cancel.is_cancelled(), "Setup cancelled");
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => return Ok(file),
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock || e.raw_os_error() == Some(33) =>
            {
                tokio::select! {_=cancel.cancelled()=>anyhow::bail!("Setup cancelled"),_=tokio::time::sleep(Duration::from_millis(150))=>{}}
            }
            Err(e) => return Err(e.into()),
        }
    }
}

pub(crate) fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    fs::write(&tmp, data)?;
    Ok(fs::rename(&tmp, path)?)
}
fn clean_private_dir(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
/// Replacing a nonempty directory requires two renames. Recover between them on a retry.
fn recover_previous(dir: &Path, stage: &Path) -> Result<()> {
    let old = stage.join("previous");
    if old.is_dir() && !dir.exists() {
        fs::rename(old, dir)?;
    }
    Ok(())
}
fn publish_directory(candidate: &Path, dir: &Path, stage: &Path) -> Result<()> {
    let old = stage.join("previous");
    clean_private_dir(&old)?;
    if dir.exists() {
        fs::rename(dir, &old)?;
    }
    if let Err(err) = fs::rename(candidate, dir) {
        recover_previous(dir, stage)?;
        return Err(err.into());
    }
    clean_private_dir(&old)?;
    Ok(())
}
pub(crate) async fn component(
    root: &Path,
    c: &Component,
    client: &Client,
    progress: &(impl Fn(ComponentStatus) + Sync),
    cancel: &CancellationToken,
) -> Result<()> {
    // Offline installs that already exist need no staging directories or network request.
    if ready(root, c) {
        progress(event(c, ComponentState::Ready, c.total()));
        return Ok(());
    }
    let stage = c.stage(root);
    let _lock = lock(&stage, cancel).await?;
    let dir = root.join(&c.directory);
    recover_previous(&dir, &stage)?;
    if !ready(root, c) {
        install_locked(root, c, client, progress, cancel).await?;
    }
    progress(event(c, ComponentState::Ready, c.total()));
    Ok(())
}

pub(crate) async fn install_locked(
    root: &Path,
    c: &Component,
    client: &Client,
    progress: &(impl Fn(ComponentStatus) + Sync),
    cancel: &CancellationToken,
) -> Result<()> {
    let stage = c.stage(root);
    let dir = root.join(&c.directory);
    fs::create_dir_all(dir.parent().context("missing installation parent")?)?;
    let candidate = stage.join("candidate");
    clean_private_dir(&stage.join("expanded"))?;
    clean_private_dir(&candidate)?;
    let mut downloaded = 0;
    let mut raw_files = Vec::new();
    let mut archives = Vec::new();
    for a in &c.files {
        let dest = a.name.as_ref().map(|n| dir.join(n));
        if let Some(existing) = &dest {
            if file_len(existing) == a.bytes
                && transfer::verify(existing, &a.sha256, cancel).await?
            {
                downloaded += a.bytes;
                continue;
            }
        }
        let part = transfer::obtain(
            client,
            a,
            &stage,
            |bytes| progress(event(c, ComponentState::Downloading, downloaded + bytes)),
            || progress(event(c, ComponentState::Verifying, downloaded + a.bytes)),
            cancel,
        )
        .await?;
        match a.format {
            Format::Raw => raw_files.push((part, dest.context("Missing raw destination")?)),
            _ => {
                archive::unpack(&part, a, &stage, cancel).await?;
                archives.push(part);
            }
        }
        downloaded += a.bytes;
    }
    ensure!(!cancel.is_cancelled(), "Setup cancelled");
    if !archives.is_empty() {
        for name in &c.commands {
            let bin = candidate.join(executable(name));
            archive::set_executable(&bin)?;
            ensure!(
                version_ok(&bin, name),
                "Could not run {} from verified archive",
                bin.display()
            );
        }
        publish_directory(&candidate, &dir, &stage)?;
    } else {
        for (part, dest) in raw_files {
            fs::create_dir_all(
                dest.parent()
                    .context("missing file installation directory")?,
            )?;
            if c.ready == Check::Tool {
                archive::set_executable(&part)?;
                #[cfg(not(windows))]
                ensure!(
                    version_ok(&part, &c.commands[0]),
                    "Downloaded tool failed its version check"
                );
            }
            fs::rename(&part, &dest)?;
        }
    }
    if c.ready == Check::Gguf {
        let first = &c.files[0];
        atomic_write(
            &dir.join(first.name.as_ref().unwrap())
                .with_extension("gguf.sha256"),
            first.sha256.as_bytes(),
        )?;
    }
    if let Some(repo) = &c.repo {
        let marker = serde_json::json!({"repo_id":repo,"bootstrap_version":1});
        atomic_write(
            &dir.join(".moxin-model-complete.json"),
            &serde_json::to_vec_pretty(&marker)?,
        )?;
    }
    if c.id == "yt-dlp" {
        let version = dir
            .file_name()
            .and_then(|p| p.to_str())
            .context("Invalid yt-dlp version")?;
        ensure!(
            version_ok(&dir.join(executable("yt-dlp")), "yt-dlp"),
            "yt-dlp failed its version check"
        );
        atomic_write(&root.join("tools/yt-dlp/current"), version.as_bytes())?;
    }
    ensure!(ready(root, c), "{} is incomplete after installation", c.id);
    for path in archives {
        fs::remove_file(path)?;
    }
    Ok(())
}
