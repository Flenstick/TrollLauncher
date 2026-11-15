//! Установка на Windows при первом запуске: exe копируется в `%LOCALAPPDATA%\Programs\TrollLauncher`,
//! создаются ярлыки в «Пуске» и на рабочем столе, лаунчер записывается в «Приложения» (HKCU, без прав
//! администратора) — оттуда же удаляется (`--uninstall`). Скачанный exe после этого можно стереть.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const UNINSTALL_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\TrollLauncher";

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

fn install_dir() -> Option<PathBuf> {
    Some(env_dir("LOCALAPPDATA")?.join("Programs").join("TrollLauncher"))
}

fn start_menu_lnk() -> Option<PathBuf> {
    Some(env_dir("APPDATA")?.join(r"Microsoft\Windows\Start Menu\Programs\TrollLauncher.lnk"))
}

fn desktop_lnk() -> Option<PathBuf> {
    Some(env_dir("USERPROFILE")?.join("Desktop").join("TrollLauncher.lnk"))
}

fn reg(args: &[&str]) {
    let _ = Command::new("reg").args(args).creation_flags(crate::sys::CREATE_NO_WINDOW).output();
}

fn shortcut(target: &Path, lnk: &Path) {
    if let Some(dir) = lnk.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut l) = mslnk::ShellLink::new(target) {
        l.set_name(Some("TrollLauncher".into()));
        l.set_icon_location(Some(target.display().to_string()));
        if let Some(wd) = target.parent() {
            l.set_working_dir(Some(wd.display().to_string()));
        }
        let _ = l.create_lnk(lnk);
    }
}

fn register(exe: &Path) {
    let e = exe.display().to_string();
    let version = env!("CARGO_PKG_VERSION");
    reg(&["add", UNINSTALL_KEY, "/v", "DisplayName", "/d", "TrollLauncher", "/f"]);
    reg(&["add", UNINSTALL_KEY, "/v", "DisplayVersion", "/d", version, "/f"]);
    reg(&["add", UNINSTALL_KEY, "/v", "Publisher", "/d", "TrollLand", "/f"]);
    reg(&["add", UNINSTALL_KEY, "/v", "DisplayIcon", "/d", &e, "/f"]);
    reg(&["add", UNINSTALL_KEY, "/v", "InstallLocation", "/d", &exe.parent().map(|p| p.display().to_string()).unwrap_or_default(), "/f"]);
    reg(&["add", UNINSTALL_KEY, "/v", "UninstallString", "/d", &format!("\"{e}\" --uninstall"), "/f"]);
    reg(&["add", UNINSTALL_KEY, "/v", "NoModify", "/t", "REG_DWORD", "/d", "1", "/f"]);
    reg(&["add", UNINSTALL_KEY, "/v", "NoRepair", "/t", "REG_DWORD", "/d", "1", "/f"]);
}

/// Вызывается в начале main. `true` — лаунчер перезапущен из папки установки, этот процесс должен выйти.
pub fn ensure_installed() -> bool {
    if std::env::var_os("TL_PORTABLE").is_some() {
        return false;
    }
    let (Ok(exe), Some(dir)) = (std::env::current_exe(), install_dir()) else { return false };
    let target = dir.join("TrollLauncher.exe");
    let same = std::fs::canonicalize(&exe).ok() == std::fs::canonicalize(&target).ok();
    if !same {
        if std::fs::create_dir_all(&dir).is_err() || std::fs::copy(&exe, &target).is_err() {
            return false; // не вышло — работаем как есть
        }
    }
    // ярлыки и запись в «Приложения» — каждый запуск (после обновления версия меняется, удалённый ярлык вернётся)
    if let Some(l) = start_menu_lnk() {
        shortcut(&target, &l);
    }
    if !same && let Some(l) = desktop_lnk() {
        shortcut(&target, &l);
    }
    register(&target);
    if same {
        return false;
    }
    Command::new(&target).args(std::env::args().skip(1)).spawn().is_ok()
}

/// `--uninstall`: убрать ярлыки, запись в «Приложениях» и сам exe. Сборки и миры остаются.
pub fn uninstall() {
    for l in [start_menu_lnk(), desktop_lnk()].into_iter().flatten() {
        let _ = std::fs::remove_file(l);
    }
    reg(&["delete", UNINSTALL_KEY, "/f"]);
    if let Some(dir) = install_dir() {
        // exe нельзя удалить, пока он запущен: удаляет cmd через пару секунд после выхода
        let script = format!("ping 127.0.0.1 -n 3 >nul & rmdir /s /q \"{}\"", dir.display());
        let _ = Command::new("cmd").args(["/c", &script]).creation_flags(crate::sys::CREATE_NO_WINDOW).spawn();
    }
}
