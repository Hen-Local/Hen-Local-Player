// Archive installation adapted from Hen-Local-Translator/hen-local-init/src/gguf.rs.
use crate::manifest::{Artifact, Format};
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path},
};
use tokio_util::sync::CancellationToken;

pub(crate) fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(path)
            .output();
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn extract_zip(file: File, a: &Artifact, dir: &Path, cancel: &CancellationToken) -> Result<()> {
    let mut archive = zip::ZipArchive::new(file)?;
    let mut found = BTreeSet::new();
    let mut buffer = vec![0; 128 * 1024];
    for index in 0..archive.len() {
        ensure!(!cancel.is_cancelled(), "Setup cancelled");
        let mut item = archive.by_index(index)?;
        if item.is_dir() {
            continue;
        }
        let path = if a.members.is_empty() {
            let rel = item.enclosed_name().context("Unsafe zip member")?;
            ensure!(
                rel.components().all(|c| matches!(c, Component::Normal(_))),
                "Unsafe zip member"
            );
            dir.join(rel)
        } else if let Some(name) = a.members.get(item.name()) {
            found.insert(item.name().to_owned());
            dir.join(name)
        } else {
            continue;
        };
        fs::create_dir_all(path.parent().context("missing parent")?)?;
        let mut out = File::create(&path)?;
        loop {
            ensure!(!cancel.is_cancelled(), "Setup cancelled");
            let n = item.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            out.write_all(&buffer[..n])?;
        }
        #[cfg(unix)]
        if let Some(mode) = item.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode & 0o777))?;
        }
    }
    ensure!(
        a.members.len() == found.len(),
        "Archive is missing an expected member"
    );
    Ok(())
}

fn extract_tar(reader: impl Read, out: &Path, cancel: &CancellationToken) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
        ensure!(!cancel.is_cancelled(), "Setup cancelled");
        let mut entry = entry?;
        ensure!(entry.unpack_in(out)?, "Unsafe tar member");
    }
    Ok(())
}

pub(crate) async fn unpack(
    part: &Path,
    a: &Artifact,
    stage: &Path,
    cancel: &CancellationToken,
) -> Result<()> {
    let (part, a, stage, cancel) = (part.to_owned(), a.clone(), stage.to_owned(), cancel.clone());
    tokio::task::spawn_blocking(move || {
        let file = File::open(&part)?;
        let extracted = stage.join("expanded");
        fs::create_dir_all(&extracted)?;
        match a.format {
            Format::Zip => extract_zip(file, &a, &extracted, &cancel)?,
            Format::Tgz => extract_tar(flate2::read::GzDecoder::new(file), &extracted, &cancel)?,
            Format::Tbz => extract_tar(bzip2::read::BzDecoder::new(file), &extracted, &cancel)?,
            Format::Raw => unreachable!(),
        }
        // Both FFmpeg archives contribute to the same candidate directory.
        let source = extracted.join(&a.prefix);
        let candidate = stage.join("candidate");
        fs::create_dir_all(&candidate)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            fs::rename(entry.path(), candidate.join(entry.file_name()))?;
        }
        fs::remove_dir_all(extracted)?;
        Ok(())
    })
    .await?
}
