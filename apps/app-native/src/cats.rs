//! Наборы котиков как в Prism Launcher: встроенные (из Prism, с праздничными вариантами)
//! и свои — из ~/.local/share/TrollLauncher/catpacks и папки котиков Prism.
//!
//! Свой набор — это картинка (png/jpg/gif/webp), папка с картинками (каждый день следующая;
//! папка `random` — случайная) или папка с `catpack.json` в формате Prism:
//! `{"name": "...", "default": "cat.png", "variants": [{"path": "xmas.png",
//!   "startTime": {"month": 12, "day": 20}, "endTime": {"month": 1, "day": 5}}]}`.

use chrono::{Datelike, Local, NaiveDate};
use std::path::{Path, PathBuf};

pub enum Src {
    /// Встроенный котик Prism (имя файла без варианта)
    Builtin(&'static str),
    File(PathBuf),
    Dir { dir: PathBuf, random: bool },
    Json { default: PathBuf, variants: Vec<((u32, u32), (u32, u32), PathBuf)> },
}

pub struct CatPack {
    pub id: String,
    pub name: String,
    pub src: Src,
    /// Откуда набор: "" — встроенный, "TrollLauncher", "Prism"
    pub origin: &'static str,
}

/// Готовая картинка для egui: uri и, для встроенных, байты.
pub struct CatImage {
    pub uri: String,
    pub bytes: Option<&'static [u8]>,
}

pub const NONE_ID: &str = "none";

pub fn own_dir() -> PathBuf {
    crate::prefs::data_dir().join("catpacks")
}
/// Скопировать картинку в свои котики; возвращает id набора (имя файла без расширения).
pub fn add_own(src: &Path) -> std::io::Result<String> {
    let dir = own_dir();
    std::fs::create_dir_all(&dir)?;
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("cat").to_string();
    let mut name = stem.clone();
    let mut n = 2;
    while BUILTIN.iter().any(|(id, _)| *id == name) || dir.join(format!("{name}.{ext}")).exists() {
        name = format!("{stem}-{n}");
        n += 1;
    }
    std::fs::copy(src, dir.join(format!("{name}.{ext}")))?;
    Ok(name)
}

fn prism_dir() -> PathBuf {
    crate::prefs::xdg_data().join("PrismLauncher/catpacks")
}

const BUILTIN: [(&str, &str); 4] = [
    ("kitteh", "Котик (из MultiMC)"),
    ("rory", "Rory ID 11 (нарисовано Ashtaka)"),
    ("rory-flat", "Rory ID 11 (плоская версия, нарисовано Ashtaka)"),
    ("teawie", "Teawie (нарисовано SympathyTea)"),
];

macro_rules! cat_files {
    ($($f:literal),*) => {
        fn builtin_bytes(name: &str) -> Option<&'static [u8]> {
            match name {
                $($f => Some(include_bytes!(concat!("../assets/cats/", $f, ".png"))),)*
                _ => None,
            }
        }
    };
}
cat_files!(
    "kitteh", "kitteh-bday", "kitteh-spooky", "kitteh-xmas",
    "rory", "rory-bday", "rory-spooky", "rory-xmas",
    "rory-flat", "rory-flat-bday", "rory-flat-spooky", "rory-flat-xmas",
    "teawie", "teawie-bday", "teawie-spooky", "teawie-xmas"
);

fn is_image(p: &Path) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    )
}

fn images_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image(p))
        .collect();
    v.sort();
    v
}

fn md(v: &serde_json::Value) -> (u32, u32) {
    let g = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(1) as u32;
    (g("month").clamp(1, 12), g("day").clamp(1, 31))
}

fn scan_dir(dir: &Path, origin: &'static str, out: &mut Vec<CatPack>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        let name_of_dir = p.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        if out.iter().any(|c| c.id == stem || c.id == name_of_dir) {
            continue;
        }
        if p.is_file() && is_image(&p) {
            out.push(CatPack { id: stem.clone(), name: stem, src: Src::File(p), origin });
        } else if p.is_dir() {
            let manifest = p.join("catpack.json");
            if let Some(v) = std::fs::read(&manifest).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) {
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or(&name_of_dir).to_string();
                let Some(def) = v.get("default").and_then(|x| x.as_str()) else { continue };
                let variants = v
                    .get("variants")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|it| {
                                let path = it.get("path")?.as_str()?;
                                Some((md(it.get("startTime")?), md(it.get("endTime")?), p.join(path)))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                out.push(CatPack { id: name_of_dir, name, src: Src::Json { default: p.join(def), variants }, origin });
            } else if !images_in(&p).is_empty() {
                let random = name_of_dir.eq_ignore_ascii_case("random");
                out.push(CatPack { id: name_of_dir.clone(), name: name_of_dir, src: Src::Dir { dir: p, random }, origin });
            }
        }
    }
}

/// Все наборы: встроенные, свои, из Prism.
pub fn scan() -> Vec<CatPack> {
    let mut v: Vec<CatPack> = BUILTIN
        .iter()
        .map(|(id, name)| CatPack { id: id.to_string(), name: name.to_string(), src: Src::Builtin(id), origin: "" })
        .collect();
    let _ = std::fs::create_dir_all(own_dir());
    scan_dir(&own_dir(), "TrollLauncher", &mut v);
    scan_dir(&prism_dir(), "Prism", &mut v);
    v
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    // 31 февраля и т.п. — последний день месяца
    (0..4).find_map(|k| NaiveDate::from_ymd_opt(y, m, d.saturating_sub(k).max(1))).unwrap()
}

fn file_uri(p: &Path) -> String {
    crate::sys::file_uri(p)
}

impl CatPack {
    /// Картинка на сегодня (праздничные варианты — как в Prism).
    pub fn image(&self, seed: u64) -> Option<CatImage> {
        let now = Local::now().date_naive();
        match &self.src {
            Src::Builtin(id) => {
                let days = |m, d| (now - date(now.year(), m, d)).num_days().abs();
                let variant = if days(12, 25) <= 4 {
                    "-xmas"
                } else if days(10, 31) <= 4 {
                    "-spooky"
                } else if days(11, 1) <= 12 {
                    "-bday"
                } else {
                    ""
                };
                let name = format!("{id}{variant}");
                let bytes = builtin_bytes(&name).or_else(|| builtin_bytes(id))?;
                Some(CatImage { uri: format!("bytes://cat/{name}.png"), bytes: Some(bytes) })
            }
            Src::File(p) => Some(CatImage { uri: file_uri(p), bytes: None }),
            Src::Dir { dir, random } => {
                let files = images_in(dir);
                if files.is_empty() {
                    return None;
                }
                let i = if *random { seed as usize } else { now.ordinal0() as usize } % files.len();
                Some(CatImage { uri: file_uri(&files[i]), bytes: None })
            }
            Src::Json { default, variants } => {
                for (s, e, p) in variants {
                    let (mut a, mut b) = (date(now.year(), s.0, s.1), date(now.year(), e.0, e.1));
                    if a > b {
                        if b < now {
                            b = date(now.year() + 1, e.0, e.1);
                        } else {
                            a = date(now.year() - 1, s.0, s.1);
                        }
                    }
                    if a <= now && now <= b {
                        return Some(CatImage { uri: file_uri(p), bytes: None });
                    }
                }
                if default.is_dir() {
                    let files = images_in(default);
                    let i = now.ordinal0() as usize % files.len().max(1);
                    return files.get(i).map(|f| CatImage { uri: file_uri(f), bytes: None });
                }
                Some(CatImage { uri: file_uri(default), bytes: None })
            }
        }
    }
}

/// Режим размещения, как CatFit в Prism + «в углу».
pub const FITS: [&str; 4] = ["В углу", "Вписать", "Заполнить", "Растянуть"];
