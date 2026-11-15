//! Синхронизация скинов с гардеробом мода sfatm: `<папка игры>/sfatm_wardrobe/wardrobe.json` + `skins/*.png`.
//! Надетый в лаунчере скин записывается в гардероб и ставится в текущий образ; sfatm при входе на сервер
//! сам применит его (SkinRestorer `/skin`) и выложит на сайт — так его видят все. Пока игра в этой папке
//! запущена, гардероб не трогаем: sfatm держит его в памяти и перезапишет.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// `Integer.toHexString(Arrays.hashCode(png)) + Integer.toHexString(png.length)` — как имя файла в sfatm.
fn java_name(png: &[u8]) -> String {
    let mut h: i32 = 1;
    for &b in png {
        h = h.wrapping_mul(31).wrapping_add(b as i8 as i32);
    }
    format!("{:x}{:x}.png", h as u32, png.len() as u32)
}

fn has_sfatm(game_dir: &Path) -> bool {
    std::fs::read_dir(game_dir.join("mods"))
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("sfatm-") && e.file_name().to_string_lossy().ends_with(".jar"))
}

/// Папки игры со sfatm: сборки лаунчера и Prism Launcher.
pub fn game_dirs(launcher_instances: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = launcher_instances.iter().filter(|d| has_sfatm(d)).cloned().collect();
    let prism = crate::sys::data_root().join("PrismLauncher").join("instances");
    for e in std::fs::read_dir(prism).into_iter().flatten().flatten() {
        for sub in ["minecraft", ".minecraft"] {
            let d = e.path().join(sub);
            if has_sfatm(&d) {
                out.push(d);
            }
        }
    }
    out
}

fn read(dir: &Path) -> Value {
    std::fs::read(dir.join("sfatm_wardrobe/wardrobe.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| json!({}))
}

fn write(dir: &Path, v: &Value) -> std::io::Result<()> {
    let w = dir.join("sfatm_wardrobe");
    std::fs::create_dir_all(&w)?;
    let tmp = w.join("wardrobe.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    std::fs::rename(tmp, w.join("wardrobe.json"))
}

/// Положить скин в гардероб и надеть в текущем образе.
pub fn put(dir: &Path, png: &[u8], name: &str, slim: bool) -> std::io::Result<()> {
    let mut v = read(dir);
    if !v.is_object() {
        v = json!({});
    }
    let file = java_name(png);
    let skins_dir = dir.join("sfatm_wardrobe/skins");
    std::fs::create_dir_all(&skins_dir)?;
    std::fs::write(skins_dir.join(&file), png)?;
    let o = v.as_object_mut().unwrap();
    let skins = o.entry("skins").or_insert_with(|| json!([]));
    if !skins.is_array() {
        *skins = json!([]);
    }
    let arr = skins.as_array_mut().unwrap();
    let id = match arr.iter_mut().find(|s| s.get("file").and_then(|f| f.as_str()) == Some(file.as_str())) {
        Some(s) => {
            s["slim"] = json!(slim);
            s.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string()
        }
        None => {
            let id = format!("s{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
            arr.insert(
                0,
                json!({"id": id, "name": name, "file": file, "source": "launcher", "slim": slim, "fav": false,
                       "added": chrono::Utc::now().timestamp_millis()}),
            );
            id
        }
    };
    let outfits = o.entry("outfits").or_insert_with(|| json!([]));
    if !outfits.is_array() || outfits.as_array().unwrap().is_empty() {
        *outfits = json!([{"name": "Образ 1", "cosm": []}]);
    }
    let n = outfits.as_array().unwrap().len() as i64;
    let cur = o.get("current").and_then(|c| c.as_i64()).unwrap_or(0).rem_euclid(n) as usize;
    o.get_mut("outfits").unwrap()[cur]["skin"] = json!(id);
    // сбросить «уже отправлено» — sfatm заново применит скин на сервере и на сайте
    o.insert("appliedServerSkin".into(), Value::Null);
    o.insert("appliedSiteSkin".into(), Value::Null);
    write(dir, &v)
}

/// Скины из гардеробов sfatm: (png, имя, тонкие руки).
pub fn list(dirs: &[PathBuf]) -> Vec<(Vec<u8>, String, bool)> {
    let mut out = vec![];
    for d in dirs {
        let v = read(d);
        for s in v.get("skins").and_then(|x| x.as_array()).into_iter().flatten() {
            let Some(file) = s.get("file").and_then(|x| x.as_str()) else { continue };
            if file.contains('/') || file.contains('\\') {
                continue;
            }
            if let Ok(png) = std::fs::read(d.join("sfatm_wardrobe/skins").join(file)) {
                let name = s.get("name").and_then(|x| x.as_str()).unwrap_or("Скин из sfatm").to_string();
                out.push((png, name, s.get("slim").and_then(|x| x.as_bool()).unwrap_or(false)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn java_hash() {
        // Arrays.hashCode(new byte[]{1, -1, 0}) = ((31+1)*31 - 1)*31 + 0 = 30721
        assert_eq!(super::java_name(&[1, 255, 0]), "78013.png");
    }
}

