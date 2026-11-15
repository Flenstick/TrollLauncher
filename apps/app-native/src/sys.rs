//! То, что на Linux и Windows делается по-разному: открыть папку или ссылку, где лежат данные.

use std::ffi::OsStr;
use std::path::PathBuf;

/// Открыть папку, файл или ссылку в программе по умолчанию.
pub fn open(target: impl AsRef<OsStr>) {
    #[cfg(windows)]
    let cmd = "explorer";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = "xdg-open";
    let _ = std::process::Command::new(cmd).arg(target).spawn();
}

/// Папка данных программ: ~/.local/share на Linux (с учётом XDG_DATA_HOME), %APPDATA% на Windows.
/// Совпадает с тем, где движок держит TrollLauncher, и с папкой Prism Launcher.
pub fn data_root() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share"))
}

/// Флаг «без окна консоли» для служебных процессов на Windows.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
