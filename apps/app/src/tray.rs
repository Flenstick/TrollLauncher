//! System tray: the tray icon and its menu (recent instances, running games),
//! "close to tray", autostart and command-line window actions.
//!
//! Settings are stored in `tray-settings.json` in the app config directory, so
//! they are available before the launcher state (database) is initialized.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::plugin::TauriPlugin;
use tauri::tray::{
    MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, Window, WindowEvent};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

const TRAY_ID: &str = "main";
const SETTINGS_FILE: &str = "tray-settings.json";
pub const AUTOSTART_ARG: &str = "--autostart";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CloseAction {
    Tray,
    Exit,
    Ask,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum LeftClickAction {
    Toggle,
    LaunchLast,
    Nothing,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AutostartMode {
    Off,
    Minimized,
    Window,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct TraySettings {
    pub enabled: bool,
    pub close_action: CloseAction,
    pub left_click: LeftClickAction,
    pub recent_count: u8,
    pub show_stop_game: bool,
    pub show_play_time: bool,
    pub autostart: AutostartMode,
    pub hidden_hint: bool,
    pub hidden_hint_shown: bool,
}

impl Default for TraySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            close_action: CloseAction::Tray,
            left_click: LeftClickAction::Toggle,
            recent_count: 5,
            show_stop_game: true,
            show_play_time: true,
            autostart: AutostartMode::Off,
            hidden_hint: true,
            hidden_hint_shown: false,
        }
    }
}

#[derive(Default)]
struct TrayState {
    settings: Mutex<TraySettings>,
    menu_signature: Mutex<String>,
}

/// Set when launched by autostart in minimized mode: the first `show_window` is skipped.
pub static START_HIDDEN: AtomicBool = AtomicBool::new(false);

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(SETTINGS_FILE))
}

fn load_settings<R: Runtime>(app: &AppHandle<R>) -> TraySettings {
    settings_path(app)
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_settings<R: Runtime>(app: &AppHandle<R>, settings: &TraySettings) {
    let Some(path) = settings_path(app) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_vec_pretty(settings) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(&path, bytes) {
                tracing::warn!("Failed to save tray settings: {e}");
            }
        }
        Err(e) => tracing::warn!("Failed to serialize tray settings: {e}"),
    }
}

fn settings<R: Runtime>(app: &AppHandle<R>) -> TraySettings {
    app.state::<TrayState>().settings.lock().unwrap().clone()
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_window("main") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    let minimized = window.is_minimized().unwrap_or(false);
    if visible && !minimized {
        let _ = window.hide();
    } else {
        show_main_window(app);
    }
}

/// Asks the frontend to launch an instance, so it goes through the same checks as the Play button.
fn request_launch<R: Runtime>(app: &AppHandle<R>, instance_id: &str) {
    let _ = app.emit("tray-launch", instance_id.to_owned());
}

fn launch_last<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Ok(mut instances) = theseus::instance::list().await else {
            return;
        };
        instances
            .sort_by_key(|meta| std::cmp::Reverse(meta.instance.last_played));
        if let Some(meta) = instances.first() {
            request_launch(&app, &meta.instance.id);
        }
    });
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours} ч {minutes} мин")
    } else {
        format!("{minutes} мин")
    }
}

/// Rebuilds the menu and tooltip when the instances, running games or settings change.
async fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let current = settings(app);
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let _ = tray.set_visible(current.enabled);
    if !current.enabled {
        return;
    }

    let mut recent = Vec::new();
    if current.recent_count > 0
        && let Ok(mut instances) = theseus::instance::list().await
    {
        instances.retain(|meta| !meta.quarantined);
        instances
            .sort_by_key(|meta| std::cmp::Reverse(meta.instance.last_played));
        recent = instances
            .into_iter()
            .take(current.recent_count as usize)
            .map(|meta| (meta.instance.id, meta.instance.name))
            .collect::<Vec<_>>();
    }

    let running = theseus::process::get_all().await.unwrap_or_default();

    use std::fmt::Write;
    let mut signature = format!("{current:?}|");
    for (id, name) in &recent {
        let _ = write!(signature, "{id}:{name};");
    }
    for process in &running {
        let _ = write!(signature, "{}:{};", process.uuid, process.instance_id);
    }

    let tooltip = match running.first() {
        Some(process) if current.show_play_time => format!(
            "TrollLauncher — {} ({})",
            process.instance_name,
            format_duration((Utc::now() - process.start_time).num_seconds())
        ),
        _ => "TrollLauncher".to_owned(),
    };
    let _ = tray.set_tooltip(Some(&tooltip));

    {
        let state = app.state::<TrayState>();
        let mut last = state.menu_signature.lock().unwrap();
        if *last == signature {
            return;
        }
        *last = signature;
    }

    match build_menu(app, &current, &recent, &running) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(e) => tracing::warn!("Failed to build tray menu: {e}"),
    }
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    current: &TraySettings,
    recent: &[(String, String)],
    running: &[theseus::data::ProcessMetadata],
) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        "Открыть TrollLauncher",
        true,
        None::<&str>,
    )?)?;

    if !recent.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        menu.append(&MenuItem::with_id(
            app,
            "recent-header",
            "Последние сборки",
            false,
            None::<&str>,
        )?)?;
        for (id, name) in recent {
            let is_running =
                running.iter().any(|process| &process.instance_id == id);
            let label = if is_running {
                format!("● {name}")
            } else {
                format!("▶ {name}")
            };
            menu.append(&MenuItem::with_id(
                app,
                format!("launch:{id}"),
                label,
                !is_running,
                None::<&str>,
            )?)?;
        }
    }

    if current.show_stop_game && !running.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
        for process in running {
            menu.append(&MenuItem::with_id(
                app,
                format!("stop:{}", process.instance_id),
                format!("Остановить: {}", process.instance_name),
                true,
                None::<&str>,
            )?)?;
        }
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "Выйти",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "open" => show_main_window(app),
        "quit" => app.exit(0),
        _ => {
            if let Some(instance_id) = id.strip_prefix("launch:") {
                request_launch(app, instance_id);
            } else if let Some(instance_id) = id.strip_prefix("stop:") {
                let instance_id = instance_id.to_owned();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = theseus::instance::kill(&instance_id).await
                    {
                        tracing::warn!(
                            "Failed to stop instance from tray: {e}"
                        );
                    }
                });
            }
        }
    }
}

fn apply_autostart<R: Runtime>(app: &AppHandle<R>, mode: AutostartMode) {
    let launcher = app.autolaunch();
    let result = match mode {
        AutostartMode::Off => launcher.disable(),
        AutostartMode::Minimized | AutostartMode::Window => launcher.enable(),
    };
    if let Err(e) = result {
        tracing::warn!("Failed to update autostart: {e}");
    }
}

fn hide_to_tray<R: Runtime>(window: &Window<R>) {
    let _ = window.hide();
    let app = window.app_handle();
    let state = app.state::<TrayState>();
    let mut current = state.settings.lock().unwrap();
    if current.hidden_hint && !current.hidden_hint_shown {
        current.hidden_hint_shown = true;
        save_settings(app, &current);
        let _ = app
            .notification()
            .builder()
            .title("TrollLauncher работает в трее")
            .body("Лаунчер свёрнут в трей. Выйти можно через меню иконки.")
            .show();
    }
}

/// Handles the window close button according to the "close action" setting.
pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != "main" {
        return;
    }
    let current = settings(window.app_handle());
    if !current.enabled {
        return;
    }
    match current.close_action {
        CloseAction::Exit => {}
        CloseAction::Tray => {
            api.prevent_close();
            hide_to_tray(window);
        }
        CloseAction::Ask => {
            api.prevent_close();
            let _ = window.emit("tray-ask-close", ());
        }
    }
}

/// Handles `--toggle`, `--show` and `--launch-last` from a second launch, so any
/// desktop or compositor shortcut can control the running launcher. Returns true if handled.
pub fn handle_cli_args<R: Runtime>(
    app: &AppHandle<R>,
    args: &[String],
) -> bool {
    if args.iter().any(|arg| arg == "--toggle") {
        toggle_main_window(app);
    } else if args.iter().any(|arg| arg == "--launch-last") {
        launch_last(app);
    } else if args.iter().any(|arg| arg == "--show") {
        show_main_window(app);
    } else {
        return false;
    }
    true
}

#[tauri::command]
fn get_settings<R: Runtime>(app: AppHandle<R>) -> TraySettings {
    settings(&app)
}

#[tauri::command]
async fn set_settings<R: Runtime>(app: AppHandle<R>, settings: TraySettings) {
    let previous_autostart = {
        let state = app.state::<TrayState>();
        let mut current = state.settings.lock().unwrap();
        let previous = current.autostart;
        *current = settings.clone();
        previous
    };
    save_settings(&app, &settings);
    if previous_autostart != settings.autostart {
        apply_autostart(&app, settings.autostart);
    }
    refresh(&app).await;
}

/// Answer from the frontend's "close or minimize?" dialog.
#[tauri::command]
async fn resolve_close<R: Runtime>(
    app: AppHandle<R>,
    to_tray: bool,
    remember: bool,
) {
    if remember {
        let updated = {
            let state = app.state::<TrayState>();
            let mut current = state.settings.lock().unwrap();
            current.close_action = if to_tray {
                CloseAction::Tray
            } else {
                CloseAction::Exit
            };
            current.clone()
        };
        save_settings(&app, &updated);
    }
    if to_tray {
        if let Some(window) = app.get_window("main") {
            hide_to_tray(&window);
        }
    } else {
        app.exit(0);
    }
}

#[tauri::command]
async fn refresh_tray<R: Runtime>(app: AppHandle<R>) {
    refresh(&app).await;
}

/// Creates the tray icon and starts the periodic menu refresh.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let loaded = load_settings(app);
    if std::env::args().any(|arg| arg == AUTOSTART_ARG)
        && loaded.autostart == AutostartMode::Minimized
    {
        START_HIDDEN.store(true, Ordering::Relaxed);
    }
    app.manage(TrayState {
        settings: Mutex::new(loaded.clone()),
        menu_signature: Mutex::default(),
    });

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("TrollLauncher")
        .menu(&build_menu(app, &loaded, &[], &[])?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                match settings(app).left_click {
                    LeftClickAction::Toggle => toggle_main_window(app),
                    LeftClickAction::LaunchLast => launch_last(app),
                    LeftClickAction::Nothing => {}
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let tray = builder.build(app)?;
    let _ = tray.set_visible(loaded.enabled);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            refresh(&app).await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    Ok(())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("troll-tray")
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            resolve_close,
            refresh_tray
        ])
        .build()
}
