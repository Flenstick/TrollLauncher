//! Автообновление TrollLauncher с GitHub-релизов (Flenstick/TrollLauncher).
//!
//! Linux — тот же формат, что у старой Tauri-версии: `trolllauncher-<ver>-x86_64.tar.gz` и `.sha256`,
//! ставится так же, как стоит текущая копия:
//! - пакет pacman (`trolllauncher-bin`) — makepkg по PKGBUILD из тега релиза и `pkexec pacman -U`;
//! - иначе исполняемый файл подменяется на месте (rename работает и у запущенной программы).
//! Windows — `TrollLauncher-<ver>-windows-x86_64.exe` и `.sha256`: запущенный exe переименовывается
//! в `.old.exe` (Windows это разрешает), новый встаёт на его место, старый удаляется при следующем запуске.
//!
//! Быстрый путь (с 2.0.6): к релизу приложены сжатый zstd исполняемый файл (`<bin>.zst`), его sha256
//! (`<bin>.sha256`) и патчи от прошлых версий (`<bin>.from-<старая>.zst`, zstd --patch-from, ~9 МБ вместо
//! 87). Лаунчер берёт патч от своей версии, иначе полный .zst, иначе старый формат. Качает в 8 потоков
//! кусками с докачкой — GitHub одним потоком отдаёт ~0,8 МБ/с, в 8 — в 6–7 раз быстрее.
//! Файлы делает `packaging/make-update-files.sh`.
//!
//! `TROLLLAUNCHER_PRETEND_VERSION=1.0.0` — притвориться старой версией для проверки.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

const REPO: &str = "Flenstick/TrollLauncher";
const PACKAGE: &str = "trolllauncher-bin";

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub current: String,
    pub version: String,
    pub notes: String,
    pub critical: bool,
    tarball_url: String,
    checksum_url: String,
    fast: Option<Fast>,
}

/// Файлы быстрого обновления: sha256 готового исполняемого файла, полный .zst и патч от текущей версии.
#[derive(Clone, Debug)]
struct Fast {
    sha_url: String,
    full_url: String,
    patch_url: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
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

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("TrollLauncher/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(20))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("http")
}

/// Клиент для куска файла: HTTP/1.1 и своё соединение у каждого потока. Через HTTP/2 все потоки
/// шли бы одним соединением, и CDN GitHub резал бы их как одну загрузку.
fn piece_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("TrollLauncher/", env!("CARGO_PKG_VERSION")))
        .http1_only()
        .connect_timeout(std::time::Duration::from_secs(20))
        .read_timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("http")
}

/// Прогресс скачивания обновления (байты), для полоски в окне.
pub static DOWNLOADED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static TOTAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Команда ручного обновления — показываем, если автоматически не вышло.
pub fn manual_hint() -> &'static str {
    if cfg!(windows) {
        "Скачайте новую версию с сайта TrollLand или со страницы релизов на GitHub и запустите её."
    } else {
        "Обновите вручную командой в терминале:\ncurl -fsSL https://raw.githubusercontent.com/Flenstick/TrollLauncher/main/install.sh | bash"
    }
}

async fn download_progress(url: &str) -> Result<Vec<u8>, String> {
    use std::sync::atomic::Ordering;
    let mut resp = client().get(url).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
    TOTAL.store(resp.content_length().unwrap_or(0), Ordering::Relaxed);
    DOWNLOADED.store(0, Ordering::Relaxed);
    let mut out = Vec::with_capacity(resp.content_length().unwrap_or(0) as usize);
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("связь оборвалась: {e}"))? {
        out.extend_from_slice(&chunk);
        DOWNLOADED.store(out.len() as u64, Ordering::Relaxed);
    }
    Ok(out)
}

fn parse_version(v: &str) -> Option<Vec<u64>> {
    let v = v.trim().trim_start_matches('v');
    let parts: Vec<u64> = v.split(['.', '-', '+']).take(3).map(|p| p.parse().ok()).collect::<Option<_>>()?;
    (parts.len() == 3).then_some(parts)
}

pub fn current_version() -> String {
    std::env::var("TROLLLAUNCHER_PRETEND_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
}

/// Новый релиз или `None`, если стоит последняя версия.
pub async fn check() -> Result<Option<UpdateInfo>, String> {
    let current = current_version();
    let release: Release = client()
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let version = release.tag_name.trim_start_matches('v').to_owned();
    let (Some(latest), Some(installed)) = (parse_version(&version), parse_version(&current)) else { return Ok(None) };
    if latest <= installed {
        return Ok(None);
    }
    let tarball = asset_name(&version);
    let find = |name: &str| release.assets.iter().find(|a| a.name == name).map(|a| a.browser_download_url.clone());
    let (Some(tarball_url), Some(checksum_url)) = (find(&tarball), find(&format!("{tarball}.sha256"))) else { return Ok(None) };
    let bin = bin_name(&version);
    let fast = match (find(&format!("{bin}.sha256")), find(&format!("{bin}.zst"))) {
        (Some(sha_url), Some(full_url)) => Some(Fast { sha_url, full_url, patch_url: find(&format!("{bin}.from-{current}.zst")) }),
        _ => None,
    };
    // первая строка описания — картинка-обложка, её в лаунчере не показываем
    let notes = release.body.replace("[critical]", "").lines().filter(|l| !l.trim_start().starts_with("![")).collect::<Vec<_>>().join("\n").trim().to_owned();
    Ok(Some(UpdateInfo { current, critical: release.body.contains("[critical]"), notes, version, tarball_url, checksum_url, fast }))
}

/// Имя файла релиза для этой системы.
fn asset_name(version: &str) -> String {
    if cfg!(windows) { format!("TrollLauncher-{version}-windows-x86_64.exe") } else { format!("trolllauncher-{version}-x86_64.tar.gz") }
}

/// Имя исполняемого файла релиза для быстрого обновления (без .zst/.sha256).
fn bin_name(version: &str) -> String {
    if cfg!(windows) { format!("TrollLauncher-{version}-windows-x86_64.exe") } else { format!("trolllauncher-{version}-x86_64.bin") }
}

const WORKERS: usize = 8;
const PIECE: u64 = 4 << 20;

/// Скачать файл в несколько потоков кусками по 4 МБ (каждый кусок докачивается при обрыве).
/// Если сервер не умеет Range — обычное скачивание одним потоком.
async fn download_fast(url: &str) -> Result<Vec<u8>, String> {
    use reqwest::{StatusCode, header};
    use std::sync::{Arc, Mutex, atomic::Ordering};
    let c = client();
    let probe = c.get(url).header(header::RANGE, "bytes=0-0").send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
    let total = (probe.status() == StatusCode::PARTIAL_CONTENT)
        .then(|| probe.headers().get(header::CONTENT_RANGE)?.to_str().ok()?.rsplit('/').next()?.parse::<u64>().ok())
        .flatten();
    let Some(total) = total.filter(|&t| t > PIECE) else {
        drop(probe);
        return download_progress(url).await;
    };
    // GitHub отвечает редиректом на подписанную ссылку CDN — берём её, чтобы не ходить через редирект на каждый кусок
    let direct = probe.url().to_string();
    drop(probe);
    TOTAL.store(total, Ordering::Relaxed);
    DOWNLOADED.store(0, Ordering::Relaxed);
    let pieces: Vec<(u64, u64)> = (0..total.div_ceil(PIECE)).map(|i| (i * PIECE, ((i + 1) * PIECE).min(total) - 1)).collect();
    let next = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let parts: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(vec![Vec::new(); pieces.len()]));
    let pieces = Arc::new(pieces);
    let mut set = tokio::task::JoinSet::new();
    for _ in 0..WORKERS {
        let (c, next, parts, pieces, direct, url) = (piece_client(), next.clone(), parts.clone(), pieces.clone(), direct.clone(), url.to_owned());
        set.spawn(async move {
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(&(start, end)) = pieces.get(i) else { return Ok(()) };
                let mut buf = Vec::with_capacity((end - start + 1) as usize);
                let mut attempt = 0;
                while (buf.len() as u64) < end - start + 1 {
                    // после ошибки — снова через github.com: подписанная ссылка могла протухнуть
                    let from = if attempt == 0 { &direct } else { &url };
                    let res: Result<(), String> = async {
                        let mut resp = c.get(from).header(header::RANGE, format!("bytes={}-{end}", start + buf.len() as u64)).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
                        if resp.status() != StatusCode::PARTIAL_CONTENT {
                            return Err("сервер не отдал кусок файла".into());
                        }
                        while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
                            let room = (end - start + 1) as usize - buf.len();
                            let chunk = &chunk[..chunk.len().min(room)];
                            buf.extend_from_slice(chunk);
                            DOWNLOADED.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                        }
                        Ok(())
                    }
                    .await;
                    if let Err(e) = res {
                        attempt += 1;
                        if attempt >= 6 {
                            return Err(format!("связь оборвалась: {e}"));
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(500 * attempt)).await;
                    }
                }
                parts.lock().unwrap()[i] = buf;
            }
        });
    }
    while let Some(r) = set.join_next().await {
        if let Err(e) = r.map_err(|e| e.to_string()).and_then(|r| r) {
            set.abort_all();
            return Err(e);
        }
    }
    let parts = std::mem::take(&mut *parts.lock().unwrap());
    let out = parts.concat();
    if out.len() as u64 != total {
        return Err("файл скачался не целиком".into());
    }
    Ok(out)
}

async fn sha_file(url: &str) -> Result<String, String> {
    let s = String::from_utf8_lossy(&download(url).await?).split_whitespace().next().unwrap_or_default().to_lowercase();
    if s.len() == 64 { Ok(s) } else { Err("битый файл контрольной суммы".into()) }
}

/// Распаковать zstd (патч — с текущим исполняемым файлом как основой).
async fn unzstd(data: Vec<u8>, base: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut out = Vec::new();
        let res = match &base {
            Some(base) => zstd::stream::read::Decoder::with_ref_prefix(&data[..], base).and_then(|mut d| {
                d.window_log_max(31)?;
                d.read_to_end(&mut out)
            }),
            None => zstd::stream::read::Decoder::new(&data[..]).and_then(|mut d| {
                d.window_log_max(31)?;
                d.read_to_end(&mut out)
            }),
        };
        res.map(|_| out).map_err(|e| format!("не удалось распаковать: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Быстрый путь: патч от текущей версии, иначе полный .zst. Возвращает готовый исполняемый файл.
async fn fetch_fast(fast: &Fast, exe: &Path) -> Result<Vec<u8>, String> {
    let expected = sha_file(&fast.sha_url).await?;
    if let Some(patch_url) = &fast.patch_url
        && let Ok(base) = tokio::fs::read(exe).await
    {
        match async { unzstd(download_fast(patch_url).await?, Some(base)).await }.await {
            Ok(bin) if format!("{:x}", Sha256::digest(&bin)) == expected => return Ok(bin),
            Ok(_) => eprintln!("[update] патч не подошёл к этой копии, качаю целиком"),
            Err(e) => eprintln!("[update] патч: {e}, качаю целиком"),
        }
    }
    let bin = unzstd(download_fast(&fast.full_url).await?, None).await?;
    if format!("{:x}", Sha256::digest(&bin)) != expected {
        return Err("контрольная сумма не совпала, обновление отменено".into());
    }
    Ok(bin)
}

async fn download(url: &str) -> Result<Vec<u8>, String> {
    Ok(client().get(url).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

#[cfg(not(windows))]
async fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    let out = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).output().await.map_err(|e| format!("{what}: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let tail: Vec<&str> = err.lines().rev().take(6).collect();
    Err(format!("{what} завершился с ошибкой ({}):\n{}", out.status, tail.into_iter().rev().collect::<Vec<_>>().join("\n")))
}

#[cfg(not(windows))]
async fn owned_by_pacman(exe: &Path) -> bool {
    Command::new("pacman")
        .arg("-Qqo")
        .arg(exe)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == PACKAGE)
}

#[cfg(not(windows))]
fn find_package(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(PACKAGE) && n.contains(".pkg.tar")))
}

/// Скачать, проверить и поставить. Новая версия запустится после `restart()`.
pub async fn install(info: &UpdateInfo) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    // pacman-установке нужен именно tarball из PKGBUILD
    #[cfg(not(windows))]
    if owned_by_pacman(&exe).await {
        let tarball = fetch_legacy(info).await?;
        return install_pacman(info, &tarball).await;
    }
    let bin = match &info.fast {
        Some(fast) => match fetch_fast(fast, &exe).await {
            Ok(bin) => bin,
            Err(e) => {
                eprintln!("[update] быстрый путь: {e}, качаю по-старому");
                legacy_binary(info).await?
            }
        },
        None => legacy_binary(info).await?,
    };
    replace_exe(&exe, &bin).await
}

/// Старый формат: tarball (Linux) или exe (Windows) с sha256 самого файла.
async fn fetch_legacy(info: &UpdateInfo) -> Result<Vec<u8>, String> {
    let data = download_fast(&info.tarball_url).await?;
    let expected = sha_file(&info.checksum_url).await?;
    if format!("{:x}", Sha256::digest(&data)) != expected {
        return Err("контрольная сумма не совпала, обновление отменено".into());
    }
    Ok(data)
}

/// Исполняемый файл из старого формата релиза.
async fn legacy_binary(info: &UpdateInfo) -> Result<Vec<u8>, String> {
    let data = fetch_legacy(info).await?;
    #[cfg(windows)]
    return Ok(data);
    #[cfg(not(windows))]
    {
        let work = work_dir().await?;
        let res = async {
            let name = format!("trolllauncher-{}-x86_64.tar.gz", info.version);
            tokio::fs::write(work.join(&name), &data).await.map_err(|e| e.to_string())?;
            run(Command::new("tar").arg("-xzf").arg(work.join(&name)).arg("-C").arg(&work), "tar").await?;
            tokio::fs::read(work.join(format!("trolllauncher-{}-x86_64", info.version)).join("trolllauncher")).await.map_err(|e| e.to_string())
        }
        .await;
        let _ = tokio::fs::remove_dir_all(&work).await;
        res
    }
}

/// Поставить новый исполняемый файл на место запущенного.
async fn replace_exe(exe: &Path, data: &[u8]) -> Result<(), String> {
    #[cfg(windows)]
    {
        let new = exe.with_extension("new.exe");
        let old = exe.with_extension("old.exe");
        tokio::fs::write(&new, data).await.map_err(|e| format!("нет доступа к папке {}: {e}", exe.parent().map(|p| p.display().to_string()).unwrap_or_default()))?;
        let _ = tokio::fs::remove_file(&old).await;
        tokio::fs::rename(exe, &old).await.map_err(|e| e.to_string())?;
        if let Err(e) = tokio::fs::rename(&new, exe).await {
            let _ = tokio::fs::rename(&old, exe).await;
            return Err(e.to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let staged = exe.with_extension("update");
        tokio::fs::write(&staged, data).await.map_err(|e| format!("нет доступа к {}: {e}", exe.display()))?;
        tokio::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).await.map_err(|e| e.to_string())?;
        tokio::fs::rename(&staged, exe).await.map_err(|e| e.to_string())
    }
}

#[cfg(not(windows))]
async fn work_dir() -> Result<PathBuf, String> {
    let work = std::env::temp_dir().join(format!("trolllauncher-update-{}", std::process::id()));
    let _ = tokio::fs::remove_dir_all(&work).await;
    tokio::fs::create_dir_all(&work).await.map_err(|e| e.to_string())?;
    Ok(work)
}

#[cfg(not(windows))]
async fn install_pacman(info: &UpdateInfo, tarball: &[u8]) -> Result<(), String> {
    let work = work_dir().await?;
    // tarball кладём в SRCDEST под тем же именем — makepkg его не перекачивает, только сверяет sha
    let name = format!("trolllauncher-{}-x86_64.tar.gz", info.version);
    tokio::fs::write(work.join(&name), tarball).await.map_err(|e| e.to_string())?;
    let res = async {
        let pkgbuild = download(&format!("https://raw.githubusercontent.com/{REPO}/v{}/packaging/arch/PKGBUILD", info.version)).await?;
        tokio::fs::write(work.join("PKGBUILD"), pkgbuild).await.map_err(|e| e.to_string())?;
        run(Command::new("makepkg").args(["-f", "--noconfirm", "--nodeps"]).env("SRCDEST", &work).current_dir(&work), "makepkg").await?;
        let pkg = find_package(&work).ok_or("makepkg не создал пакет")?;
        // pkexec ждёт пароль в окне polkit; если окна нет (голый оконный менеджер) — ждал бы вечно
        match tokio::time::timeout(std::time::Duration::from_secs(180), run(Command::new("pkexec").args(["pacman", "-U", "--noconfirm"]).arg(&pkg), "pacman")).await {
            Ok(r) => r,
            Err(_) => Err("не дождался ввода пароля (нет окна polkit?)".into()),
        }
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&work).await;
    res
}

/// Перезапустить лаунчер новой версией. Движок держит блокировку папки, поэтому новая копия
/// сначала ждёт, пока эта закроется (`TL_WAIT_PID`, см. `wait_previous`).
pub fn restart() -> ! {
    if let Ok(exe) = std::env::current_exe() {
        // на Linux после подмены файла /proc/self/exe указывает на «… (deleted)»
        let path = PathBuf::from(exe.to_string_lossy().trim_end_matches(" (deleted)").to_string());
        let _ = std::process::Command::new(path).env("TL_WAIT_PID", std::process::id().to_string()).stdin(Stdio::null()).spawn();
    }
    std::process::exit(0);
}

fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .creation_flags(crate::sys::CREATE_NO_WINDOW)
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
    }
    #[cfg(not(windows))]
    {
        Path::new(&format!("/proc/{pid}")).exists()
    }
}

/// Вызывается при старте: дождаться выхода прошлой копии после обновления (не дольше 15 с)
/// и убрать оставшийся от обновления `.old.exe`.
pub fn wait_previous() {
    if let Some(pid) = std::env::var("TL_WAIT_PID").ok().and_then(|p| p.parse::<u32>().ok()) {
        let start = std::time::Instant::now();
        while alive(pid) && start.elapsed() < std::time::Duration::from_secs(15) {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        // SAFETY: в самом начале main, до запуска других потоков
        unsafe { std::env::remove_var("TL_WAIT_PID") };
    }
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(exe.with_extension("old.exe"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(parse_version("2.0.1") > parse_version("2.0.0"));
        assert!(parse_version("v2.0.0") > parse_version("1.1.1"));
        assert_eq!(parse_version("1.2"), None);
    }
}

