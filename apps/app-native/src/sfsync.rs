//! Двусторонняя синхронизация скинов с гардеробом sfatm (запись — sfskin.rs).
//!
//! Лаунчер → игра: «Надеть» пишет гардероб сборок, где игра не идёт; запущенным кладёт запрос
//! `launcher.json` — sfatm ≥ 1.32.124 применяет скин на лету. Если запрос не принят (старый мод),
//! скин записывается в гардероб после выхода из игры.
//! Игра → лаунчер: изменение `wardrobe.json` ловится по mtime — новые скины добавляются
//! в библиотеку, а скин текущего образа отмечается надетым у аккаунта, которым играют.

use crate::backend::{Backend, Msg};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use theseus::prelude::*;

/// mtime wardrobe.json, при котором синхронизация уже прошла.
static SEEN: std::sync::LazyLock<Mutex<HashMap<PathBuf, SystemTime>>> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
pub(crate) static SYNC: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Ник из командной строки игры (--username рядом с --gameDir) — чтобы отметить скин у нужного аккаунта.
#[cfg(target_os = "linux")]
fn game_nicks() -> HashMap<PathBuf, String> {
    let mut out = HashMap::new();
    for e in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Ok(cmd) = std::fs::read(e.path().join("cmdline")) else { continue };
        let arg = |name: &[u8]| -> Option<String> {
            let i = cmd.windows(name.len()).position(|w| w == name)?;
            let rest = &cmd[i + name.len()..];
            let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
            Some(String::from_utf8_lossy(&rest[..end]).to_string())
        };
        if let (Some(dir), Some(nick)) = (arg(b"--gameDir\0"), arg(b"--username\0")) {
            out.insert(canon(Path::new(&dir)), nick);
        }
    }
    out
}

impl Backend {
    /// Синхронизация с гардеробами sfatm. Вызывается из tick раз в несколько секунд:
    /// проверка mtime дешёвая, чтение гардероба — только при изменениях.
    pub fn sfatm_watch(&self) {
        self.task(|tx| async move {
			let Ok(_watch) = SYNC.try_lock() else { return };
            let dirs = crate::sfskin::game_dirs(&Self::instance_dirs().await);
            let busy: Vec<PathBuf> = Self::busy_dirs().await.iter().map(|p| canon(p)).collect();

			for d in &dirs {
				if !busy.contains(&canon(d)) {
					let _ = crate::sfskin::apply_pending(d);
				}
			}

            // игра → лаунчер: только папки, где wardrobe.json изменился
            let Ok(lib) = theseus::skin_library::get().await else { return };
            let mut fresh: Vec<(PathBuf, bool)> = vec![]; // (папка, уже видели раньше)
            {
                let mut seen = SEEN.lock().unwrap();
                for d in &dirs {
                    let Ok(mt) = std::fs::metadata(d.join("sfatm_wardrobe/wardrobe.json")).and_then(|m| m.modified()) else { continue };
                    let known = seen.get(d).is_some_and(|t| *t == mt);
                    if known {
                        continue;
                    }
                    fresh.push((d.clone(), seen.contains_key(d)));
                    seen.insert(d.clone(), mt);
                }
            }
            if fresh.is_empty() {
                return;
            }

            let known: HashSet<String> = lib.skins.iter().map(|s| s.id.clone()).collect();
            let mut added: Vec<(String, String)> = vec![];
            let mut worn: Vec<(PathBuf, String, bool)> = vec![]; // (папка, скин, папка уже видели)
            for (d, seen_before) in &fresh {
                for (png, name, slim) in crate::sfskin::list(std::slice::from_ref(d)) {
                    let v = if slim { theseus::skin_library::LibrarySkinVariant::Slim } else { theseus::skin_library::LibrarySkinVariant::Classic };
                    if let Ok(s) = theseus::skin_library::add(png, name, Some(v), None, "sfatm".into()).await
                        && !known.contains(&s.id)
                        && !added.iter().any(|(id, _)| *id == s.id)
                    {
                        added.push((s.id, s.name));
                    }
                }
                if let Some((png, name, slim)) = crate::sfskin::worn(d) {
                    let v = if slim { theseus::skin_library::LibrarySkinVariant::Slim } else { theseus::skin_library::LibrarySkinVariant::Classic };
                    if let Ok(s) = theseus::skin_library::add(png, name, Some(v), None, "sfatm".into()).await {
                        if !known.contains(&s.id) && !added.iter().any(|(id, _)| *id == s.id) {
                            added.push((s.id.clone(), s.name));
                        }
                        worn.push((d.clone(), s.id, *seen_before));
                    }
                }
            }

            // надетый в игре скин — надетым и в лаунчере (у аккаунта, которым играют)
            let users = minecraft_auth::users().await.unwrap_or_default();
            let def = minecraft_auth::get_default_user().await.ok().flatten();
            #[cfg(target_os = "linux")]
            let nicks = game_nicks();
            #[cfg(not(target_os = "linux"))]
            let nicks: HashMap<PathBuf, String> = HashMap::new();
            let mut eq_changed = false;
            let mut eq_notify = false;
            for (d, id, seen_before) in worn {
				let path = canon(&d);
				let running = busy.contains(&path);
                let acc = nicks
                    .get(&path)
                    .and_then(|n| users.iter().find(|u| u.offline_profile.name.eq_ignore_ascii_case(n)))
                    .map(|u| u.offline_profile.id)
                    .or(def);
                let Some(acc) = acc else { continue };
                let cur = theseus::skin_library::get().await.ok().and_then(|l| l.equipped.get(&acc.to_string()).cloned());
				if !running && (!busy.is_empty() || (!seen_before && cur.is_some())) {
					continue;
				}
                if cur.as_deref() != Some(id.as_str()) && theseus::skin_library::set_equipped(acc.to_string(), id).await.is_ok() {
                    eq_changed = true;
                    eq_notify |= seen_before; // при первом скане не шумим — это не смена скина в игре
                }
            }

            if !added.is_empty() {
                let text = if added.len() == 1 { format!("Скин «{}» из sfatm добавлен в библиотеку", added[0].1) } else { format!("Из sfatm добавлено скинов: {}", added.len()) };
                let _ = tx.send(Msg::Toast(text, false));
            }
            if eq_notify {
                let _ = tx.send(Msg::Toast("Скин из игры отмечен надетым".into(), false));
            }
            if !added.is_empty() || eq_changed {
                let _ = tx.send(Msg::ReloadSkins);
            }
        });
    }
}