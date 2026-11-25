//! The store supplies a personal HTTPS feed. Only the feed receives that link.
//! Packages are fetched independently and verified with the embedded publisher key.
use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use hlp_pipeline::store;
use reqwest::{Client, StatusCode, Url};
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
    pub_date: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    platforms: HashMap<String, Platform>,
}
#[derive(Deserialize)]
struct Feed {
    releases: Vec<Release>,
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
        .context("Enter a full HTTPS update link from the store")
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

    fn summary(self, current: &Version) -> Option<AppRelease> {
        let version = stable_version(&self.version)?;
        let installable = version.cmp_precedence(current).is_gt() && self.package().is_some();
        Some(AppRelease {
            version,
            published_at: self.pub_date.unwrap_or_default(),
            notes: self.notes.unwrap_or_default(),
            installable,
        })
    }
}

#[cfg(any(target_os = "macos", windows))]
fn access_entry() -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(
        "com.henlocal.player.update-access",
        "personal-feed-v1",
    )?)
}
fn saved_access() -> Result<Option<String>> {
    #[cfg(any(target_os = "macos", windows))]
    match access_entry()?.get_password() {
        Ok(url) => Ok(Some(url)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.into()),
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Ok(None)
    }
}

#[tauri::command]
pub async fn app_update_access() -> Result<bool, String> {
    saved_access()
        .map(|value| value.is_some())
        .map_err(|e| e.to_string())
}

fn configure_access(url: &str) -> Result<()> {
    let url = url.trim();
    if !url.is_empty() {
        https_url(url)?;
    }
    let dir = store::data_root().join("updates");
    let _lock = operation_lock(&dir)?;
    #[cfg(any(target_os = "macos", windows))]
    {
        if url.is_empty() {
            match access_entry()?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(err) => return Err(err.into()),
            }
        } else {
            access_entry()?.set_password(url)?;
        }
        if let Some(previous) = pending(&dir) {
            forget(&dir, &previous);
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        bail!("App updates are not supported on this platform")
    }
}

#[tauri::command]
pub async fn set_app_update_access(url: String) -> Result<(), String> {
    configure_access(&url).map_err(|e| e.to_string())
}

async fn fetch_feed() -> Result<Feed> {
    let url = https_url(&saved_access()?.context("UPDATE_ACCESS_REQUIRED")?)?;
    // A purchaser link can contain a secret in its path or query. Don't forward
    // it across redirects, include it in a Referer, or surface URLs in errors.
    let client = Client::builder()
        .https_only(true)
        .referer(false)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Hen-Local-Player/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(reqwest::Error::without_url)?;
    match response.status() {
        StatusCode::OK => Ok(response.json().await.map_err(reqwest::Error::without_url)?),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::GONE => {
            bail!("UPDATE_ACCESS_DENIED")
        }
        StatusCode::NOT_FOUND => bail!("UPDATE_FEED_UNAVAILABLE"),
        StatusCode::TOO_MANY_REQUESTS => bail!("UPDATE_RATE_LIMITED"),
        status => bail!("The update service returned {status}"),
    }
}

async fn releases(current: &Version) -> Result<Vec<AppRelease>> {
    let mut history = fetch_feed()
        .await?
        .releases
        .into_iter()
        .filter_map(|r| r.summary(current))
        .collect::<Vec<_>>();
    history.sort_by(|a, b| b.version.cmp(&a.version));
    history.dedup_by(|a, b| a.version == b.version);
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
    let release = fetch_feed()
        .await?
        .releases
        .into_iter()
        .find(|r| r.version == selected)
        .context("This release is unavailable for your purchase")?;
    let package = release
        .package()
        .context("No signed update for this platform")?;
    let manifest = serde_json::json!({
        "version": version,
        "platforms": { (platform_key()): {"url": OFFLINE_URL, "signature": package.signature} }
    });
    let mut update = restore(&app, &version, manifest.clone())?;
    // Only the short-lived artifact link goes to the file host. No customer or
    // GitHub credentials are sent as headers or saved to the staged manifest.
    update.download_url = https_url(&package.url)?;
    // Tauri sets up its rustls provider in check(). A private feed restores the
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
    fn private_feed_keeps_only_stable_versions_and_requires_complete_platform_assets() {
        let current = Version::new(0, 1, 0);
        let release = |version: &str| Release {
            version: version.to_owned(),
            notes: None,
            pub_date: None,
            platforms: HashMap::new(),
        };
        assert!(release("1.0.0-beta.1").summary(&current).is_none());
        assert!(release("not-semver").summary(&current).is_none());
        assert!(!release("0.0.9").summary(&current).unwrap().installable);
        let mut upcoming = release("0.10.0");
        assert!(upcoming.package().is_none());
        let artifact = Platform {
            url: "https://downloads.example/movie?temporary=1".into(),
            size: 700,
            sha256: "ab".repeat(32),
            signature: "signature".into(),
        };
        upcoming.platforms.insert(platform_key().into(), artifact);
        assert!(upcoming.summary(&current).unwrap().installable);
        assert!(https_url("http://example.org/?buyer=test").is_err());
        assert!(https_url("https://user:pass@example.org/").is_err());
        assert!(https_url("https://example.org/#secret").is_err());
    }
}
