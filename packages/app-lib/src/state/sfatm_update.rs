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

/// Проверить и обновить sfatm в `instance_dir/mods`. `Ok(None)` — обновлять нечего.
pub async fn update(instance_dir: &Path) -> crate::Result<Option<Updated>> {
    let mods = instance_dir.join("mods");
    let Some((old_path, old_ver)) = installed(&mods) else { return Ok(None) };
    let url = std::env::var("SFATM_UPDATE_URL").unwrap_or_else(|_| MANIFEST_URL.to_string());
    let http = reqwest::Client::builder()
        .user_agent(concat!("TrollLauncher/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(6))
        .build()
        .map_err(err)?;
    let resp = http.get(&url).timeout(Duration::from_secs(10)).send().await.map_err(err)?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None); // релизов ещё нет
    }
    let m: Manifest = resp.error_for_status().map_err(err)?.json().await.map_err(err)?;
    if !newer(&m.version, &old_ver) {
        return Ok(None);
    }
    if version_of(&m.file).is_none() || m.file.contains('/') || m.file.contains('\\') {
        return Err(err(format!("странное имя файла в latest.json: {}", m.file)));
    }
    tracing::info!("sfatm: обновление {old_ver} → {}", m.version);
    let bytes = http.get(&m.url).timeout(Duration::from_secs(600)).send().await.map_err(err)?.error_for_status().map_err(err)?.bytes().await.map_err(err)?;
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
    tokio::fs::rename(&part, mods.join(&m.file)).await.map_err(err)?;
    Ok(Some(Updated { from: old_ver, to: m.version }))
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
