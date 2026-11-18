//! Автообновление нашего мода sfatm: перед запуском сборки, где лежит `mods/sfatm-*.jar`,
//! смотрим `latest.json` (GitHub Flenstick/sfatm-releases) и, если там версия новее,
//! скачиваем jar, сверяем sha512 и меняем файл. Старый jar уходит в `removed-mods/`.
//! Без сети или при любой ошибке игра просто запускается со старой версией.
//!
//! latest.json: {"version": "1.32.116", "file": "sfatm-1.32.116.jar", "url": "https://…", "sha512": "…", "size": 123}
//! Адрес можно подменить переменной SFATM_UPDATE_URL (для проверки).

use reqwest::header::{ACCEPT_ENCODING, CONTENT_RANGE, RANGE};
use serde::Deserialize;
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

pub const MANIFEST_URL: &str = "https://github.com/Flenstick/sfatm-releases/releases/latest/download/latest.json";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const MAX_DOWNLOAD_STREAMS: u64 = 6;
const MIN_RANGE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Deserialize, Debug, Clone)]
pub struct Manifest {
    pub version: String,
    pub file: String,
    pub url: String,
    pub sha512: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Updated {
    pub from: String,
    pub to: String,
}

fn version_of(file_name: &str) -> Option<String> {
    let v = file_name.strip_prefix("sfatm-")?.strip_suffix(".jar")?;
    v.chars().next().filter(|c| c.is_ascii_digit())?;
    Some(v.to_string())
}

/// Сравнение версий по числам: 1.32.9 < 1.32.10.
pub fn newer(a: &str, b: &str) -> bool {
    let nums = |s: &str| {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|x| !x.is_empty())
            .map(|x| x.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    nums(a) > nums(b)
}

/// Установленный sfatm в папке модов (выключенный `.jar.disabled` не трогаем).
fn installed(mods: &Path) -> Option<(PathBuf, String)> {
    let mut best: Option<(PathBuf, String)> = None;
    for e in std::fs::read_dir(mods).ok()?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(v) = version_of(&name)
            && best.as_ref().is_none_or(|(_, b)| newer(&v, b))
        {
            best = Some((e.path(), v));
        }
    }
    best
}

fn err(e: impl std::fmt::Display) -> crate::Error {
    crate::ErrorKind::OtherError(format!("sfatm: {e}")).into()
}

fn client() -> crate::Result<&'static reqwest::Client> {
    static CLIENT: OnceLock<Result<reqwest::Client, reqwest::Error>> =
        OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .user_agent(concat!(
                    "TrollLauncher/",
                    env!("CARGO_PKG_VERSION")
                ))
                .connect_timeout(Duration::from_secs(6))
                .read_timeout(Duration::from_secs(15))
                .http1_only()
                .build()
        })
        .as_ref()
        .map_err(err)
}

/// Последний latest.json и когда его взяли: один запрос на все сборки раз в 10 минут.
static CACHE: Mutex<Option<(std::time::Instant, Option<Manifest>)>> =
    Mutex::const_new(None);
static INSTALL_LOCK: Mutex<()> = Mutex::const_new(());

/// latest.json с GitHub (`Ok(None)` — релизов ещё нет). `fresh` — мимо кэша.
pub async fn manifest(fresh: bool) -> crate::Result<Option<Manifest>> {
    let mut cache = CACHE.lock().await;
    if !fresh
        && let Some((t, m)) = cache.as_ref()
        && t.elapsed() < Duration::from_secs(600)
    {
        return Ok(m.clone());
    }
    let url = std::env::var("SFATM_UPDATE_URL")
        .unwrap_or_else(|_| MANIFEST_URL.to_string());
    let resp = client()?
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(err)?;
    let m = if resp.status() == reqwest::StatusCode::NOT_FOUND {
        None
    } else {
        let m: Manifest = resp
            .error_for_status()
            .map_err(err)?
            .json()
            .await
            .map_err(err)?;
        validate_manifest(&m)?;
        Some(m)
    };
    *cache = Some((std::time::Instant::now(), m.clone()));
    Ok(m)
}

fn validate_manifest(m: &Manifest) -> crate::Result<()> {
    if version_of(&m.file).as_deref() != Some(m.version.as_str())
        || m.file.contains(['/', '\\', ':'])
    {
        return Err(err(format!(
            "странное имя файла в latest.json: {}",
            m.file
        )));
    }
    let hash = m.sha512.trim();
    if hash.len() != 128 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(err("неверный SHA-512 в latest.json"));
    }
    Ok(())
}

async fn read_download(
    mut response: reqwest::Response,
    expected_size: u64,
    mut on_chunk: impl FnMut(u64),
) -> crate::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(err)? {
        if expected_size != 0
            && (bytes.len() + chunk.len()) as u64 > expected_size
        {
            return Err(err("ответ сервера больше ожидаемого размера"));
        }
        bytes.extend_from_slice(&chunk);
        on_chunk(chunk.len() as u64);
    }
    if expected_size != 0 && bytes.len() as u64 != expected_size {
        return Err(err(format!(
            "размер не совпал: {} вместо {expected_size}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

async fn download_ranges(
    m: &Manifest,
    streams: u64,
    progress: &impl Fn(u64, u64),
) -> crate::Result<Vec<u8>> {
    let client = client()?;
    let received = AtomicU64::new(0);
    let part_size = m.size.div_ceil(streams);
    let requests = (0..streams).map(|part| {
        let received = &received;
        async move {
            let start = part * part_size;
            let end = start.saturating_add(part_size).min(m.size) - 1;
            let response = client
                .get(&m.url)
                .header(RANGE, format!("bytes={start}-{end}"))
                .header(ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(err)?;
            let expected_range = format!("bytes {start}-{end}/{}", m.size);
            if response.status() != reqwest::StatusCode::PARTIAL_CONTENT
                || response
                    .headers()
                    .get(CONTENT_RANGE)
                    .and_then(|value| value.to_str().ok())
                    != Some(expected_range.as_str())
            {
                return Err(err("сервер не вернул запрошенный диапазон"));
            }
            read_download(response, end - start + 1, |bytes| {
                let done = received.fetch_add(bytes, Ordering::Relaxed) + bytes;
                progress(done, m.size);
            })
            .await
        }
    });
    let parts = futures::future::try_join_all(requests).await?;
    Ok(parts.concat())
}

fn verified(bytes: &[u8], m: &Manifest) -> bool {
    (m.size == 0 || bytes.len() as u64 == m.size)
        && format!("{:x}", Sha512::digest(bytes))
            .eq_ignore_ascii_case(m.sha512.trim())
}

async fn download(
    m: &Manifest,
    progress: &impl Fn(u64, u64),
) -> crate::Result<Vec<u8>> {
    let streams = (m.size / MIN_RANGE_BYTES).min(MAX_DOWNLOAD_STREAMS);
    if streams >= 2 {
        progress(0, m.size);
        match download_ranges(m, streams, progress).await {
            Ok(bytes) if verified(&bytes, m) => return Ok(bytes),
            Ok(_) => tracing::warn!(
                "sfatm: SHA-512 не совпал после загрузки по частям"
            ),
            Err(e) => tracing::debug!("sfatm: переход к обычной загрузке: {e}"),
        }
    }
    let response = client()?
        .get(&m.url)
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    let total = response.content_length().unwrap_or(m.size);
    progress(0, total);
    let mut received = 0;
    let bytes = read_download(response, m.size, |bytes| {
        received += bytes;
        progress(received, total);
    })
    .await?;
    if !verified(&bytes, m) {
        return Err(err("контрольная сумма не совпала — файл не поставлен"));
    }
    Ok(bytes)
}

async fn release_bytes(
    m: &Manifest,
    progress: &impl Fn(u64, u64),
) -> crate::Result<Vec<u8>> {
    let state = super::State::get_if_initialized();
    let hash = m.sha512.trim().to_ascii_lowercase();
    if let Some(state) = &state
        && let Ok(Some(cached)) =
            state.content_store.lookup(Some(&hash), None).await
        && let Ok(bytes) = tokio::fs::read(&cached.path).await
        && verified(&bytes, m)
    {
        let size = bytes.len() as u64;
        progress(size, size);
        return Ok(bytes);
    }
    let bytes = tokio::time::timeout(DOWNLOAD_TIMEOUT, download(m, progress))
        .await
        .map_err(err)??;
    if let Some(state) = &state
        && let Err(e) = state.content_store.store_bytes(&bytes).await
    {
        tracing::debug!("sfatm: не удалось сохранить релиз в кэше: {e}");
    }
    Ok(bytes)
}

/// Версия sfatm в сборке (`instance_dir/mods`), если мод там есть.
pub fn installed_version(instance_dir: &Path) -> Option<String> {
    installed(&instance_dir.join("mods")).map(|(_, v)| v)
}

/// Скачать и поставить версию из манифеста. `progress(скачано, всего)` зовётся по ходу загрузки.
/// Файл сначала пишется рядом как `.part`, проверяется sha512 и только потом подменяет старый jar.
pub async fn install(
    instance_dir: &Path,
    m: &Manifest,
    progress: impl Fn(u64, u64),
) -> crate::Result<Option<Updated>> {
    let _install = INSTALL_LOCK.lock().await;
    let mods = instance_dir.join("mods");
    let Some((old_path, old_ver)) = installed(&mods) else {
        return Ok(None);
    };
    if !newer(&m.version, &old_ver) {
        return Ok(None);
    }
    validate_manifest(m)?;
    tracing::info!(
        "sfatm: обновление {old_ver} → {} в {}",
        m.version,
        instance_dir.display()
    );
    let bytes = release_bytes(m, &progress).await?;
    let part = tempfile::Builder::new()
        .prefix(".sfatm-")
        .suffix(".part")
        .tempfile_in(&mods)
        .map_err(err)?
        .into_temp_path();
    tokio::fs::write(&part, &bytes).await.map_err(err)?;
    let removed = instance_dir.join("removed-mods");
    tokio::fs::create_dir_all(&removed).await.map_err(err)?;
    let old_name = old_path
        .file_name()
        .map(|n| n.to_owned())
        .unwrap_or_default();
    let backup = removed.join(&old_name);
    tokio::fs::rename(&old_path, &backup).await.map_err(err)?;
    if let Err(e) = part.persist_noclobber(mods.join(&m.file)) {
        // не оставлять сборку без мода: вернуть старый jar на место
        tokio::fs::rename(&backup, &old_path)
            .await
            .map_err(|restore| {
                err(format!(
                    "{e}; прежний JAR находится в {}: {restore}",
                    backup.display()
                ))
            })?;
        return Err(err(e));
    }
    Ok(Some(Updated {
        from: old_ver,
        to: m.version.clone(),
    }))
}

/// Проверить и обновить sfatm в `instance_dir/mods`. `Ok(None)` — обновлять нечего.
pub async fn update(instance_dir: &Path) -> crate::Result<Option<Updated>> {
    let Some(old_ver) = installed_version(instance_dir) else {
        return Ok(None);
    };
    match manifest(false).await? {
        Some(m) if newer(&m.version, &old_ver) => {
            install(instance_dir, &m, |_, _| {}).await
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(newer("1.32.116", "1.32.115"));
        assert!(newer("1.32.100", "1.32.99"));
        assert!(!newer("1.32.115", "1.32.115"));
        assert_eq!(
            version_of("sfatm-1.32.115.jar").as_deref(),
            Some("1.32.115")
        );
        assert_eq!(version_of("sfatm-1.32.115.jar.disabled"), None);
    }
}
