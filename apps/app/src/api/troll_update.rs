//! TrollLauncher self-update from GitHub releases.
//!
//! The Tauri updater only handles AppImages on Linux, while TrollLauncher ships
//! as a tarball installed by pacman (`trolllauncher-bin`) or copied by hand.
//! This checks the latest GitHub release and installs it the same way the
//! current copy was installed:
//! - owned by pacman: builds the package from the release PKGBUILD with
//!   makepkg and installs it with `pkexec pacman -U` (one password prompt);
//! - anywhere else: swaps the executable in place.
//!
//! Every release carries `trolllauncher-<ver>-x86_64.tar.gz` and a
//! `.tar.gz.sha256` file next to it; the tarball is checked before use.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::plugin::TauriPlugin;
use tauri::Runtime;
use tauri_plugin_http::reqwest;
use tokio::process::Command;

use crate::api::Result;

const REPO: &str = "Flenstick/TrollLauncher";
const PACKAGE: &str = "trolllauncher-bin";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("troll-update")
        .invoke_handler(tauri::generate_handler![check, install, restart])
        .build()
}

#[derive(Serialize, Clone, Debug)]
pub struct UpdateInfo {
    pub current: String,
    pub version: String,
    pub notes: String,
    pub published: String,
    pub critical: bool,
    tarball_url: String,
    checksum_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn other(message: impl Into<String>) -> crate::api::TheseusSerializableError {
    theseus::Error::from(theseus::ErrorKind::OtherError(message.into())).into()
}

fn network(error: reqwest::Error) -> crate::api::TheseusSerializableError {
    other(format!("Не удалось связаться с GitHub: {error}"))
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(format!("TrollLauncher/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("http client")
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.trim_start_matches('v').split(['.', '-', '+']);
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next().unwrap_or("0").parse().ok()?,
    ))
}

/// Returns the newer release, or `None` when this copy is up to date.
#[tauri::command]
async fn check<R: Runtime>(app: tauri::AppHandle<R>) -> Result<Option<UpdateInfo>> {
    // TROLLLAUNCHER_PRETEND_VERSION lets a test build act as an older release.
    let current = std::env::var("TROLLLAUNCHER_PRETEND_VERSION")
        .unwrap_or_else(|_| app.package_info().version.to_string());
    let release: Release = async {
        client()
            .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }
    .await
    .map_err(network)?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let version = release.tag_name.trim_start_matches('v').to_owned();
    let (Some(latest), Some(installed)) =
        (parse_version(&version), parse_version(&current))
    else {
        return Ok(None);
    };
    if latest <= installed {
        return Ok(None);
    }

    let tarball_name = format!("trolllauncher-{version}-x86_64.tar.gz");
    let find = |name: &str| {
        release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .map(|asset| asset.browser_download_url.clone())
    };
    let (Some(tarball_url), Some(checksum_url)) =
        (find(&tarball_name), find(&format!("{tarball_name}.sha256")))
    else {
        return Ok(None);
    };

    Ok(Some(UpdateInfo {
        current,
        critical: release.body.contains("[critical]"),
        notes: release.body.replace("[critical]", "").trim().to_owned(),
        version,
        published: release.published_at,
        tarball_url,
        checksum_url,
    }))
}

async fn download(url: &str) -> Result<Vec<u8>> {
    let bytes = async {
        client().get(url).send().await?.error_for_status()?.bytes().await
    }
    .await
    .map_err(network)?;
    Ok(bytes.to_vec())
}

async fn run(command: &mut Command, what: &str) -> Result<()> {
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| other(format!("{what}: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let tail: Vec<&str> = stderr.lines().rev().take(6).collect();
    Err(other(format!(
        "{what} завершился с ошибкой ({}):\n{}",
        output.status,
        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
    )))
}

async fn owned_by_pacman(exe: &Path) -> bool {
    Command::new("pacman")
        .arg("-Qqo")
        .arg(exe)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .is_ok_and(|out| {
            out.status.success()
                && String::from_utf8_lossy(&out.stdout).trim() == PACKAGE
        })
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum InstallMethod {
    Pacman,
    Replaced,
}

/// Downloads, verifies and installs the update. The new version runs after
/// `restart`.
#[tauri::command]
async fn install<R: Runtime>(app: tauri::AppHandle<R>) -> Result<InstallMethod> {
    let info = check(app).await?.ok_or_else(|| other("Обновлений нет"))?;
    let tarball = download(&info.tarball_url).await?;
    let expected = String::from_utf8_lossy(&download(&info.checksum_url).await?)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_lowercase();
    let actual: String = Sha256::digest(&tarball)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if expected.len() != 64 || actual != expected {
        return Err(other(
            "Контрольная сумма архива не совпала, обновление отменено",
        ));
    }

    let work = tempfile::tempdir()?;
    let tarball_name = format!("trolllauncher-{}-x86_64.tar.gz", info.version);
    tokio::fs::write(work.path().join(&tarball_name), &tarball).await?;

    let exe = std::env::current_exe()?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);

    if owned_by_pacman(&exe).await {
        let pkgbuild = download(&format!(
            "https://raw.githubusercontent.com/{REPO}/v{}/packaging/arch/PKGBUILD",
            info.version
        ))
        .await?;
        tokio::fs::write(work.path().join("PKGBUILD"), pkgbuild).await?;
        run(
            Command::new("makepkg")
                .args(["-f", "--noconfirm", "--nodeps"])
                .env("SRCDEST", work.path())
                .current_dir(work.path()),
            "makepkg",
        )
        .await?;
        let package = find_package(work.path())
            .ok_or_else(|| other("makepkg не создал пакет"))?;
        run(
            Command::new("pkexec")
                .args(["pacman", "-U", "--noconfirm"])
                .arg(&package),
            "pacman",
        )
        .await?;
        return Ok(InstallMethod::Pacman);
    }

    run(
        Command::new("tar")
            .arg("-xzf")
            .arg(work.path().join(&tarball_name))
            .arg("-C")
            .arg(work.path()),
        "tar",
    )
    .await?;
    let new_binary = work
        .path()
        .join(format!("trolllauncher-{}-x86_64", info.version))
        .join("trolllauncher");
    let staged = exe.with_extension("update");
    tokio::fs::copy(&new_binary, &staged)
        .await
        .map_err(|e| other(format!("Нет доступа к {}: {e}", exe.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
            .await?;
    }
    // rename() swaps the file even while the old binary is running.
    tokio::fs::rename(&staged, &exe).await?;
    Ok(InstallMethod::Replaced)
}

fn find_package(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|path| {
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(PACKAGE) && n.contains(".pkg.tar"))
    })
}

#[tauri::command]
fn restart<R: Runtime>(app: tauri::AppHandle<R>) {
    app.restart();
}
