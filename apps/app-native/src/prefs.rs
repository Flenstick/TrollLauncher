//! Настройки интерфейса (тема, акцент, котики, масштаб) — ~/.local/share/TrollLauncher/native-ui.json.
//! При первом запуске котик берётся из настроек Prism Launcher.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Prefs {
    pub theme: String,
    /// Свой акцент (индекс в ACCENTS) или `None` — акцент темы
    pub accent: Option<usize>,
    pub scale: f32,
    pub animations: bool,
    /// id набора котиков или "none"
    pub cat: String,
    /// 0..=100
    pub cat_opacity: u8,
    /// индекс в cats::FITS
    pub cat_fit: usize,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: "trollland".into(),
            accent: None,
            scale: 1.0,
            animations: true,
            cat: "kitteh".into(),
            cat_opacity: 100,
            cat_fit: 0,
        }
    }
}

pub fn xdg_data() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share"))
}
pub fn data_dir() -> PathBuf {
    xdg_data().join(crate::backend::APP_ID)
}
fn file() -> PathBuf {
    data_dir().join("native-ui.json")
}

pub fn load() -> Prefs {
    if let Some(p) = std::fs::read(file()).ok().and_then(|b| serde_json::from_slice(&b).ok()) {
        return p;
    }
    let mut p = Prefs::default();
    // Котик из Prism: BackgroundCat, CatOpacity, CatFit, TheCat
    if let Ok(cfg) = std::fs::read_to_string(xdg_data().join("PrismLauncher/prismlauncher.cfg")) {
        for line in cfg.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "BackgroundCat" if !v.trim().is_empty() => p.cat = v.trim().to_string(),
                "CatOpacity" => p.cat_opacity = v.trim().parse().unwrap_or(100),
                "CatFit" => {
                    p.cat_fit = match v.trim() {
                        "fill" => 2,
                        "strech" | "stretch" => 3,
                        _ => 1,
                    }
                }
                "TheCat" if v.trim() == "false" => p.cat = crate::cats::NONE_ID.into(),
                _ => {}
            }
        }
    }
    p
}

pub fn save(p: &Prefs) {
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(b) = serde_json::to_vec_pretty(p) {
        let tmp = file().with_extension("json.tmp");
        if std::fs::write(&tmp, b).is_ok() {
            let _ = std::fs::rename(tmp, file());
        }
    }
}
