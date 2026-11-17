//! Автообновление нашего мода sfatm: перед запуском сборки, где лежит `mods/sfatm-*.jar`,
//! смотрим `latest.json` (GitHub Flenstick/sfatm-releases) и, если там версия новее,
//! скачиваем jar, сверяем sha512 и меняем файл. Старый jar уходит в `removed-mods/`.
//! Без сети или при любой ошибке игра просто запускается со старой версией.
//!
//! latest.json: {"version": "1.32.116", "file": "sfatm-1.32.116.jar", "url": "https://…", "sha512": "…", "size": 123}
//! Адрес можно подменить переменной SFATM_UPDATE_URL (для проверки).

use serde::Deserialize;
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MANIFEST_URL: &str = "https://github.com/Flenstick/sfatm-releases/releases/latest/download/latest.json";

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
    let nums = |s: &str| s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
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

fn client() -> crate::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("TrollLauncher/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(6))
        .build()
        .map_err(err)
}

/// Последний latest.json и когда его взяли: один запрос на все сборки раз в 10 минут.
static CACHE: std::sync::Mutex<Option<(std::time::Instant, Option<Manifest>)>> = std::sync::Mutex::new(None);

/// latest.json с GitHub (`Ok(None)` — релизов ещё нет). `fresh` — мимо кэша.
pub async fn manifest(fresh: bool) -> crate::Result<Option<Manifest>> {
    if !fresh
        && let Some((t, m)) = CACHE.lock().unwrap().as_ref()
        && t.elapsed() < Duration::from_secs(600)
    {
        return Ok(m.clone());
    }
    let url = std::env::var("SFATM_UPDATE_URL").unwrap_or_else(|_| MANIFEST_URL.to_string());
    let resp = client()?.get(&url).timeout(Duration::from_secs(10)).send().await.map_err(err)?;
    let m = if resp.status() == reqwest::StatusCode::NOT_FOUND {
        None
    } else {
        let m: Manifest = resp.error_for_status().map_err(err)?.json().await.map_err(err)?;
        if version_of(&m.file).is_none() || m.file.contains('/') || m.file.contains('\\') {
            return Err(err(format!("странное имя файла в latest.json: {}", m.file)));
        }
        Some(m)
    };
    *CACHE.lock().unwrap() = Some((std::time::Instant::now(), m.clone()));
    Ok(m)
}

/// Версия sfatm в сборке (`instance_dir/mods`), если мод там есть.
pub fn installed_version(instance_dir: &Path) -> Option<String> {
    installed(&instance_dir.join("mods")).map(|(_, v)| v)
}

/// Скачать и поставить версию из манифеста. `progress(скачано, всего)` зовётся по ходу загрузки.
/// Файл сначала пишется рядом как `.part`, проверяется sha512 и только потом подменяет старый jar.
pub async fn install(instance_dir: &Path, m: &Manifest, progress: impl Fn(u64, u64)) -> crate::Result<Option<Updated>> {
    let mods = instance_dir.join("mods");
    let Some((old_path, old_ver)) = installed(&mods) else { return Ok(None) };
    if !newer(&m.version, &old_ver) {
        return Ok(None);
    }
    tracing::info!("sfatm: обновление {old_ver} → {} в {}", m.version, instance_dir.display());
    let mut resp = client()?.get(&m.url).timeout(Duration::from_secs(600)).send().await.map_err(err)?.error_for_status().map_err(err)?;
    let total = resp.content_length().unwrap_or(m.size);
    let mut bytes = Vec::with_capacity(total as usize);
    progress(0, total);
    while let Some(chunk) = resp.chunk().await.map_err(err)? {
        bytes.extend_from_slice(&chunk);
        progress(bytes.len() as u64, total);
    }
    if m.size > 0 && bytes.len() as u64 != m.size {
        return Err(err(format!("размер не совпал: {} вместо {}", bytes.len(), m.size)));
    }
    let hash = format!("{:x}", Sha512::digest(&bytes));
    if !hash.eq_ignore_ascii_case(m.sha512.trim()) {
        return Err(err("контрольная сумма не совпала — файл не поставлен"));
    }
    let part = mods.join(format!(".{}.part", m.file));
    tokio::fs::write(&part, &bytes).await.map_err(err)?;
    let removed = instance_dir.join("removed-mods");
    let _ = tokio::fs::create_dir_all(&removed).await;
    let old_name = old_path.file_name().map(|n| n.to_owned()).unwrap_or_default();
    if tokio::fs::rename(&old_path, removed.join(&old_name)).await.is_err() {
        let _ = tokio::fs::remove_file(&old_path).await;
    }
    if let Err(e) = tokio::fs::rename(&part, mods.join(&m.file)).await {
        // не оставлять сборку без мода: вернуть старый jar на место
        let _ = tokio::fs::rename(removed.join(&old_name), &old_path).await;
        let _ = tokio::fs::remove_file(&part).await;
        return Err(err(e));
    }
    Ok(Some(Updated { from: old_ver, to: m.version.clone() }))
}

/// Проверить и обновить sfatm в `instance_dir/mods`. `Ok(None)` — обновлять нечего.
pub async fn update(instance_dir: &Path) -> crate::Result<Option<Updated>> {
    let Some(old_ver) = installed_version(instance_dir) else { return Ok(None) };
    match manifest(false).await? {
        Some(m) if newer(&m.version, &old_ver) => install(instance_dir, &m, |_, _| {}).await,
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
        assert_eq!(version_of("sfatm-1.32.115.jar").as_deref(), Some("1.32.115"));
        assert_eq!(version_of("sfatm-1.32.115.jar.disabled"), None);
    }
}
