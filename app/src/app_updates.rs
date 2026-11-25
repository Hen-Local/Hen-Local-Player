use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use hlp_pipeline::store;
use reqwest::{Client, Response, StatusCode, Url};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};

// The updater requires a configured endpoint even to restore a manifest locally.
// restore_update never contacts this URL. It is also the non-secret URL used in
// the staged manifest once the signed artifact has been downloaded.
const REPOSITORY: &str = include_str!("../release-repository.txt");
const OFFLINE_URL: &str = "https://offline.invalid/package";

#[derive(Deserialize)]
struct Platform {
    url: String,
    signature: String,
    size: u64,
    sha256: String,
}

#[derive(Deserialize)]
struct Release {
    version: String,
    #[serde(default)]
    platforms: HashMap<String, Platform>,
}
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRelease {
    version: Version,
    published_at: String,
    notes: String,
    installable: bool,
}

fn stable_version(value: &str) -> Option<Version> {
    Version::parse(value)
        .ok()
        .filter(|v| v.pre.is_empty() && v.build.is_empty())
}

fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-aarch64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => "unsupported",
    }
}

fn https_url(input: &str) -> Result<Url> {
    Url::parse(input.trim())
        .ok()
        .filter(|u| {
            u.scheme() == "https"
                && u.host_str().is_some()
                && u.username().is_empty()
                && u.password().is_none()
                && u.fragment().is_none()
        })
        .context("Invalid HTTPS release asset URL")
}

impl Release {
    fn package(&self) -> Option<&Platform> {
        let p = self.platforms.get(platform_key())?;
        (p.size > 0
            && p.sha256.len() == 64
            && p.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            && !p.signature.is_empty()
            && https_url(&p.url).is_ok())
        .then_some(p)
    }
}

impl GitHubRelease {
    fn summary(self, current: &Version) -> Option<AppRelease> {
        if self.draft || self.prerelease {
            return None;
        }
        let version = stable_version(self.tag_name.strip_prefix('v')?)?;
        let suffix = match platform_key() {
            "darwin-aarch64" => Some("macos-arm64.app.tar.gz"),
            "windows-x86_64" => Some("windows-x64-setup.exe"),
            _ => None,
        };
        let has = |name: &str| self.assets.iter().any(|a| a.name == name);
        let installable = version > *current
            && has("latest.json")
            && suffix.is_some_and(|suffix| {
                let filename = format!("Hen-Local-Player-{version}-{suffix}");
                has(&filename) && has(&format!("{filename}.sig"))
            });
        Some(AppRelease {
            version,
            published_at: self.published_at.unwrap_or_default(),
            notes: self.body.unwrap_or_default(),
            installable,
        })
    }
}

fn release_client() -> Result<Client> {
    Ok(Client::builder()
        .https_only(true)
        .referer(false)
        .user_agent(concat!("Hen-Local-Player/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()?)
}

async fn get_release_data(client: &Client, url: &str) -> Result<Response> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(reqwest::Error::without_url)?;
    match response.status() {
        StatusCode::OK => Ok(response),
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => bail!("UPDATE_RATE_LIMITED"),
        StatusCode::NOT_FOUND => bail!("UPDATE_FEED_UNAVAILABLE"),
        status => bail!("The update service returned {status}"),
    }
}

async fn releases(current: &Version) -> Result<Vec<AppRelease>> {
    let client = release_client()?;
    let endpoint = format!(
        "https://api.github.com/repos/{}/releases",
        REPOSITORY.trim()
    );
    let mut history = Vec::new();
    for page in 1.. {
        let url = format!("{endpoint}?per_page=100&page={page}");
        let response = get_release_data(&client, &url).await?;
        let batch: Vec<GitHubRelease> = response.json().await?;
        let last = batch.len() < 100;
        history.extend(
            batch
                .into_iter()
                .filter_map(|release| release.summary(current)),
        );
        if last {
            break;
        }
    }
    history.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(history)
}

#[tauri::command]
pub async fn list_app_releases(app: AppHandle) -> Result<Vec<AppRelease>, String> {
    releases(&app.package_info().version)
        .await
        .map_err(|e| e.to_string())
}

// The metadata holds the version-bound Tauri signature. Even if this local cache
// is modified, Tauri verifies the package again with our embedded public key
// and the currently installed version before touching the application.
#[derive(Deserialize, Serialize)]
struct Pending {
    version: Version,
    size: u64,
    manifest: Value,
}

impl Pending {
    fn package(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.package", self.version))
    }

    fn available(&self, dir: &Path, current: &Version) -> bool {
        self.version.cmp_precedence(current).is_gt()
            && self.manifest.get("version").and_then(Value::as_str)
                == Some(self.version.to_string().as_str())
            && fs::metadata(self.package(dir))
                .is_ok_and(|meta| meta.len() == self.size && self.size != 0)
    }
}

fn pending(dir: &Path) -> Option<Pending> {
    store::read(&dir.join("pending.json"))
}

fn operation_lock(dir: &Path) -> Result<File> {
    fs::create_dir_all(dir)?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("operation.lock"))?;
    lock.try_lock_exclusive()
        .context("Another app update is already in progress")?;
    Ok(lock)
}

fn forget(dir: &Path, item: &Pending) {
    let _ = fs::remove_file(dir.join("pending.json"));
    let _ = fs::remove_file(item.package(dir));
}

#[tauri::command]
pub async fn prepared_app_release(app: AppHandle) -> Option<String> {
    let dir = store::data_root().join("updates");
    let item = pending(&dir)?;
    let current = &app.package_info().version;
    if item.available(&dir, current) {
        return Some(item.version.to_string());
    }
    // Release staged files once an updater or manual install has overtaken them.
    if let Ok(_lock) = operation_lock(&dir) {
        if let Some(item) = pending(&dir).filter(|p| !p.available(&dir, current)) {
            forget(&dir, &item);
        }
    }
    None
}

fn restore<R: tauri::Runtime>(
    app: &AppHandle<R>,
    version: &Version,
    manifest: Value,
) -> Result<Update> {
    let builder = app
        .updater_builder()
        .endpoints(vec![Url::parse(OFFLINE_URL)?])?
        .configure_client(|client| client.https_only(true).referer(false));
    // The Windows installer exits this process directly, bypassing RunEvent::Exit.
    #[cfg(windows)]
    let builder = {
        use tauri::Manager;
        let app = app.clone();
        builder.on_before_exit(move || {
            tauri::async_runtime::block_on(app.state::<crate::state::Desktop>().shutdown());
            app.cleanup_before_exit();
        })
    };
    let update = builder
        .build()?
        .restore_update(manifest)?
        .context("This release is already installed or older")?;
    ensure!(
        update.version == version.to_string(),
        "The manifest does not match the selected release"
    );
    Ok(update)
}

#[derive(Serialize, Clone)]
struct Progress<'a> {
    version: &'a str,
    downloaded: u64,
    total: u64,
}

async fn download(app: AppHandle, selected: &str) -> Result<()> {
    let version = stable_version(selected).context("Invalid app release version")?;
    ensure!(
        version.cmp_precedence(&app.package_info().version).is_gt(),
        "Select a newer release"
    );
    let dir = store::data_root().join("updates");
    let _lock = operation_lock(&dir)?;
    let previous = pending(&dir);
    if previous
        .as_ref()
        .is_some_and(|p| p.version == version && p.available(&dir, &app.package_info().version))
    {
        return Ok(());
    }
    let url = format!(
        "https://github.com/{}/releases/download/v{version}/latest.json",
        REPOSITORY.trim()
    );
    let release: Release = get_release_data(&release_client()?, &url)
        .await?
        .json()
        .await?;
    ensure!(
        release.version == selected,
        "The release tag and manifest version differ"
    );
    let package = release
        .package()
        .context("No signed update for this platform")?;
    let manifest = serde_json::json!({
        "version": version,
        "platforms": { (platform_key()): {"url": OFFLINE_URL, "signature": package.signature} }
    });
    let mut update = restore(&app, &version, manifest.clone())?;
    update.download_url = https_url(&package.url)?;
    // Tauri sets up its rustls provider in check(). This client restores the
    // selected manifest directly and therefore has to initialize it for download.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    update.timeout = Some(Duration::from_secs(30 * 60));
    let mut downloaded = 0;
    let mut last = Instant::now();
    let bytes = update
        .download(
            |chunk, _| {
                downloaded += chunk as u64;
                if last.elapsed() >= Duration::from_millis(150) || downloaded == package.size {
                    crate::events::emit(
                        &app,
                        "app-update://progress",
                        Progress {
                            version: selected,
                            downloaded,
                            total: package.size,
                        },
                    );
                    last = Instant::now();
                }
            },
            || {},
        )
        .await
        .map_err(|e| match e {
            tauri_plugin_updater::Error::Reqwest(e) => anyhow::Error::from(e.without_url()),
            other => anyhow::Error::from(other),
        })?;
    let expected = package.sha256.clone();
    let item = Pending {
        version,
        size: package.size,
        manifest,
    };
    tokio::task::spawn_blocking(move || -> Result<()> {
        ensure!(
            bytes.len() as u64 == item.size
                && hex::encode(Sha256::digest(&bytes)).eq_ignore_ascii_case(&expected),
            "The downloaded update does not match the release size or SHA-256 digest"
        );
        store::atomic_write(&item.package(&dir), &bytes)?;
        store::write_json(&dir.join("pending.json"), &item)?;
        if let Some(old) = previous.filter(|p| p.version != item.version) {
            let _ = fs::remove_file(old.package(&dir));
        }
        Ok(())
    })
    .await??;
    Ok(())
}

#[tauri::command]
pub async fn download_app_release(app: AppHandle, version: String) -> Result<(), String> {
    download(app, &version).await.map_err(|e| e.to_string())
}

fn install(app: &AppHandle, version: &str) -> Result<()> {
    let dir = store::data_root().join("updates");
    let _lock = operation_lock(&dir)?;
    let item = pending(&dir)
        .filter(|p| {
            p.version.to_string() == version && p.available(&dir, &app.package_info().version)
        })
        .context("Download the selected release before installing")?;

    #[cfg(target_os = "macos")]
    {
        // In cargo/tauri dev the executable lives directly in target/debug. A
        // macOS updater may only replace an actual installed application bundle.
        let exe = std::env::current_exe()?;
        let bundle = exe
            .ancestors()
            .nth(3)
            .context("Start the packaged macOS app to install updates")?;
        ensure!(
            bundle.extension().is_some_and(|s| s == "app")
                && exe.parent() == Some(bundle.join("Contents/MacOS").as_path()),
            "Start the packaged macOS app to install updates"
        );
    }
    let update =
        restore(app, &item.version, item.manifest.clone()).inspect_err(|_| forget(&dir, &item))?;
    let bytes = fs::read(item.package(&dir))?;
    // Discard a corrupt cache so the same version can be downloaded again.
    if let Err(error) = update.verify(&bytes) {
        forget(&dir, &item);
        return Err(error.into());
    }
    update.install(bytes)?;
    app.request_restart();
    Ok(())
}

#[tauri::command]
pub async fn install_app_release(app: AppHandle, version: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || install(&app, &version))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_history_marks_only_new_signed_platform_packages_installable() {
        let make = |tag: &str, preview: bool| GitHubRelease {
            tag_name: tag.into(),
            draft: false,
            prerelease: preview,
            body: None,
            published_at: None,
            assets: [
                "latest.json",
                "Hen-Local-Player-0.2.0-macos-arm64.app.tar.gz",
                "Hen-Local-Player-0.2.0-macos-arm64.app.tar.gz.sig",
                "Hen-Local-Player-0.2.0-windows-x64-setup.exe",
                "Hen-Local-Player-0.2.0-windows-x64-setup.exe.sig",
            ]
            .iter()
            .map(|name| GitHubAsset {
                name: name.to_string(),
            })
            .collect(),
        };
        let current = Version::new(0, 1, 1);
        assert!(make("v0.2.0-beta.1", false).summary(&current).is_none());
        assert!(make("v0.2.0", true).summary(&current).is_none());
        assert!(!make("v0.1.0", false).summary(&current).unwrap().installable);
        let mut missing = make("v0.2.0", false);
        missing.assets.remove(0);
        assert!(!missing.summary(&current).unwrap().installable);
        assert_eq!(
            make("v0.2.0", false).summary(&current).unwrap().installable,
            platform_key() != "unsupported"
        );
    }
}
