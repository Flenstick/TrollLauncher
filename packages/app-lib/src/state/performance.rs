//! TrollLauncher performance settings: tuned JVM flags, Feral GameMode and
//! MangoHud wrappers applied to every game launch.
//!
//! Stored in `performance.json` next to the app database, so no migration is
//! needed and unknown/missing fields fall back to defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SETTINGS_FILE: &str = "performance.json";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum JvmPreset {
    /// Only what the user typed in Java arguments.
    Off,
    /// Aikar's G1 flags — the same set the TrollLand Prism instance uses.
    G1,
    /// Generational ZGC: almost no GC pauses, needs Java 17+ (falls back to G1).
    Zgc,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct PerformanceSettings {
    pub jvm_preset: JvmPreset,
    /// `-Xms` equal to `-Xmx`: the whole heap is taken at start.
    pub full_heap_at_start: bool,
    pub gamemode: bool,
    pub mangohud: bool,
    /// Frontend-only: pause launcher animations while a game is running.
    pub pause_ui_while_playing: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            jvm_preset: JvmPreset::G1,
            full_heap_at_start: false,
            gamemode: true,
            mangohud: false,
            pause_ui_while_playing: true,
        }
    }
}

const G1_FLAGS: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:+AlwaysPreTouch",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:G1HeapRegionSize=8M",
    "-XX:G1ReservePercent=20",
    "-XX:G1HeapWastePercent=5",
    "-XX:G1MixedGCCountTarget=4",
    "-XX:InitiatingHeapOccupancyPercent=15",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:SurvivorRatio=32",
    "-XX:+PerfDisableSharedMem",
    "-XX:MaxTenuringThreshold=1",
    "-XX:+UseStringDeduplication",
];

/// Initial heap when the full heap is not taken at start (Prism uses the same idea).
const DEFAULT_MIN_HEAP_MB: u32 = 2048;

fn settings_path() -> Option<PathBuf> {
    crate::state::DirectoryInfo::global_handle_if_ready()
        .map(|dirs| dirs.settings_dir.join(SETTINGS_FILE))
}

impl PerformanceSettings {
    pub async fn get() -> Self {
        let Some(path) = settings_path() else {
            return Self::default();
        };
        tokio::fs::read(path)
            .await
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub async fn set(&self) -> crate::Result<()> {
        let Some(path) = settings_path() else {
            return Err(crate::ErrorKind::OtherError(
                "Launcher directories are not ready".to_string(),
            )
            .into());
        };
        let bytes = serde_json::to_vec_pretty(self)?;
        crate::util::io::write(&path, bytes).await?;
        Ok(())
    }

    /// JVM flags to put before the user's own Java arguments (later flags win,
    /// so anything the user typed still overrides the preset).
    pub fn jvm_args(
        &self,
        user_args: &[String],
        max_memory_mb: u32,
        java_major: u32,
    ) -> Vec<String> {
        let mut args = Vec::new();

        let user_sets_min_heap =
            user_args.iter().any(|arg| arg.starts_with("-Xms"));
        if !user_sets_min_heap {
            let min = if self.full_heap_at_start {
                max_memory_mb
            } else {
                DEFAULT_MIN_HEAP_MB.min(max_memory_mb)
            };
            args.push(format!("-Xms{min}M"));
        }

        // Choosing two collectors makes the JVM refuse to start.
        let user_picks_gc = user_args
            .iter()
            .any(|arg| arg.starts_with("-XX:+Use") && arg.ends_with("GC"));
        if user_picks_gc {
            return args;
        }

        let preset = match self.jvm_preset {
            JvmPreset::Zgc if java_major < 17 => JvmPreset::G1,
            preset => preset,
        };
        match preset {
            JvmPreset::Off => {}
            JvmPreset::G1 => {
                args.extend(G1_FLAGS.iter().map(|flag| flag.to_string()));
            }
            JvmPreset::Zgc => {
                args.push("-XX:+UseZGC".to_string());
                // Generational mode is the default (and only) one since Java 23.
                if java_major < 23 {
                    args.push("-XX:+ZGenerational".to_string());
                }
                args.extend(
                    [
                        "-XX:+AlwaysPreTouch",
                        "-XX:+DisableExplicitGC",
                        "-XX:+PerfDisableSharedMem",
                        "-XX:+UseStringDeduplication",
                    ]
                    .iter()
                    .map(|flag| flag.to_string()),
                );
            }
        }
        args
    }

    /// Programs to run the game through, in order, before any user wrapper.
    pub fn wrapper_prefix(&self) -> Vec<String> {
        let mut prefix = Vec::new();
        if cfg!(target_os = "linux") {
            if self.gamemode && which("gamemoderun").is_some() {
                prefix.push("gamemoderun".to_string());
            }
            if self.mangohud && which("mangohud").is_some() {
                prefix.push("mangohud".to_string());
            }
        }
        prefix
    }
}

/// Which optional tools are installed, for the settings page.
#[derive(Serialize, Clone, Debug)]
pub struct PerformanceTools {
    pub gamemode: bool,
    pub mangohud: bool,
}

pub fn available_tools() -> PerformanceTools {
    let linux = cfg!(target_os = "linux");
    PerformanceTools {
        gamemode: linux && which("gamemoderun").is_some(),
        mangohud: linux && which("mangohud").is_some(),
    }
}

fn which(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(program))
            .find(|path| path.is_file())
    })
}
