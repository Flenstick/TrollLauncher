use crate::api::Result;
use tauri::plugin::TauriPlugin;
use theseus::prelude::{PerformanceSettings, PerformanceTools, performance};

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("troll-perf")
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            get_tools
        ])
        .build()
}

#[tauri::command]
pub async fn get_settings() -> Result<PerformanceSettings> {
    Ok(PerformanceSettings::get().await)
}

#[tauri::command]
pub async fn set_settings(settings: PerformanceSettings) -> Result<()> {
    Ok(settings.set().await?)
}

#[tauri::command]
pub async fn get_tools() -> Result<PerformanceTools> {
    Ok(performance::available_tools())
}
