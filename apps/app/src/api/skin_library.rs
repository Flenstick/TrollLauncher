//! Skin library commands, plus putting a skin on an Ely.by account.
//!
//! Ely.by has no public API for changing skins, so the launcher drives the
//! ely.by website in its own window (the user signs in there once, cookies are
//! kept): it uploads the PNG to the catalog and wears it, the same requests the
//! site itself makes. The script reports back by navigating to RESULT_PATH.

use std::collections::HashMap;
use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{Duration, Utc};
use serde::Serialize;
use tauri::plugin::TauriPlugin;
use tauri::{Manager, Runtime, WebviewUrl, WebviewWindow};
use theseus::skin_library::{
    self, ExtraCapes, FetchedSkin, LibrarySkin, LibrarySkinVariant, SkinLibrary,
};

use crate::api::Result;

const ELYBY_WINDOW: &str = "elyby-web";
const ELYBY_ADD_URL: &str = "https://ely.by/skins/add";
const ELYBY_LOGIN_URL: &str = "https://ely.by/authorization/login";
const RESULT_PATH: &str = "/__trolllauncher_result";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("troll-skins")
        .invoke_handler(tauri::generate_handler![
            get_library,
            add_skin,
            update_skin,
            remove_skin,
            set_folders,
            set_equipped,
            get_texture,
            export_skin,
            fetch_by_nick,
            fetch_url,
            extra_capes,
            elyby_apply,
            elyby_open,
        ])
        .build()
}

#[tauri::command]
async fn get_library() -> Result<SkinLibrary> {
    Ok(skin_library::get().await?)
}

#[tauri::command]
async fn add_skin(
    texture: Vec<u8>,
    name: String,
    variant: Option<LibrarySkinVariant>,
    folder: Option<String>,
    source: String,
) -> Result<LibrarySkin> {
    Ok(skin_library::add(texture, name, variant, folder, source).await?)
}

#[tauri::command]
async fn update_skin(skin: LibrarySkin) -> Result<()> {
    Ok(skin_library::update(skin).await?)
}

#[tauri::command]
async fn remove_skin(id: String) -> Result<()> {
    Ok(skin_library::remove(&id).await?)
}

#[tauri::command]
async fn set_folders(
    folders: Vec<String>,
    renamed: HashMap<String, String>,
) -> Result<()> {
    Ok(skin_library::set_folders(folders, renamed).await?)
}

#[tauri::command]
async fn set_equipped(account: String, id: String) -> Result<()> {
    Ok(skin_library::set_equipped(account, id).await?)
}

#[tauri::command]
async fn get_texture(id: String) -> Result<Vec<u8>> {
    Ok(skin_library::texture(&id).await?.to_vec())
}

#[tauri::command]
async fn export_skin(id: String, path: PathBuf) -> Result<()> {
    Ok(skin_library::export(&id, path).await?)
}

#[tauri::command]
async fn fetch_by_nick(nick: String, source: String) -> Result<FetchedSkin> {
    Ok(skin_library::fetch_by_nick(&nick, &source).await?)
}

#[tauri::command]
async fn fetch_url(url: String) -> Result<FetchedSkin> {
    Ok(skin_library::fetch_url(&url).await?)
}

#[tauri::command]
async fn extra_capes(nick: String) -> Result<ExtraCapes> {
    Ok(skin_library::extra_capes(&nick).await?)
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ElybyApplyResult {
    /// Skin uploaded and worn.
    Done,
    /// Not signed in on ely.by: the window is shown with the login page.
    NeedLogin,
    /// Something unexpected: the window is shown so the user can finish by hand.
    Manual { message: String },
    /// The user closed the window.
    Cancelled,
}

fn ely_window<R: Runtime>(
    app: &tauri::AppHandle<R>,
    url: &str,
) -> Result<WebviewWindow<R>> {
    let url: tauri::Url = url.parse().expect("static ely.by url");
    if let Some(window) = app.get_webview_window(ELYBY_WINDOW) {
        window.navigate(url)?;
        return Ok(window);
    }
    Ok(tauri::WebviewWindowBuilder::new(
        app,
        ELYBY_WINDOW,
        WebviewUrl::External(url),
    )
    .title("Ely.by — TrollLauncher")
    .inner_size(1000.0, 760.0)
    .min_inner_size(500.0, 500.0)
    .visible(false)
    .center()
    .build()?)
}

fn show(window: &WebviewWindow<impl Runtime>) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Opens the ely.by window (e.g. to sign in) without doing anything else.
#[tauri::command]
async fn elyby_open<R: Runtime>(
    app: tauri::AppHandle<R>,
    page: Option<String>,
) -> Result<()> {
    let url = match page.as_deref() {
        Some("login") => ELYBY_LOGIN_URL,
        _ => ELYBY_ADD_URL,
    };
    let window = ely_window(&app, url)?;
    show(&window);
    Ok(())
}

fn upload_script(texture: &[u8], slim: bool) -> String {
    format!(
        r#"(async () => {{
  const done = (result) => {{
    location.href = '{RESULT_PATH}?r=' + encodeURIComponent(JSON.stringify(result))
  }}
  try {{
    if (document.readyState !== 'complete') await new Promise((r) => addEventListener('load', r, {{ once: true }}))
    if (document.querySelector('a[href*="/authorization/login"]') && !document.querySelector('a[href*="/authorization/logout"]')) {{
      return done({{ login: true }})
    }}
    const headers = {{ 'X-Requested-With': 'XMLHttpRequest', Accept: 'application/json' }}
    const bytes = Uint8Array.from(atob('{texture}'), (c) => c.charCodeAt(0))
    const form = new FormData()
    form.append('file', new Blob([bytes], {{ type: 'image/png' }}), 'skin.png')
    const upload = await fetch('/api/legacy/skins', {{ method: 'POST', body: form, headers, credentials: 'include' }})
    const text = await upload.text()
    let body = null
    try {{ body = JSON.parse(text) }} catch {{}}
    if (upload.status === 401 || upload.status === 403) return done({{ login: true }})
    if (!body) return done({{ error: 'upload ' + upload.status + ': ' + text.slice(0, 200) }})
    const flat = JSON.stringify(body)
    const match = flat.match(/skins\/s(\d+)/) || flat.match(/"(?:skinId|skin_id|id)"\s*:\s*"?(\d+)/)
    if (!match) return done({{ error: 'upload answer: ' + flat.slice(0, 300), url: body.url || null }})
    const skinId = match[1]
    const wear = await fetch('/api/legacy/users/skin', {{
      method: 'PUT', body: new URLSearchParams({{ skinId }}), headers, credentials: 'include',
    }})
    let wearBody = null
    try {{ wearBody = await wear.json() }} catch {{}}
    if (!wear.ok || (wearBody && wearBody.error && !String(wearBody.error).includes('success'))) {{
      return done({{ error: 'wear ' + wear.status + ': ' + JSON.stringify(wearBody).slice(0, 200), url: '/skins/s' + skinId }})
    }}
    done({{ ok: true, skinId, slim: {slim} }})
  }} catch (e) {{
    done({{ error: String(e) }})
  }}
}})()"#,
        texture = BASE64.encode(texture),
    )
}

/// Uploads a library skin to the ely.by catalog and wears it on the signed-in
/// ely.by account.
#[tauri::command]
async fn elyby_apply<R: Runtime>(
    app: tauri::AppHandle<R>,
    id: String,
    slim: bool,
) -> Result<ElybyApplyResult> {
    let texture = skin_library::texture(&id).await?;
    let window = ely_window(&app, ELYBY_ADD_URL)?;

    // Wait for the add page to load, then run the upload in its context.
    let start = Utc::now();
    let mut injected = false;
    while Utc::now() - start < Duration::seconds(60) {
        if window.title().is_err() {
            return Ok(ElybyApplyResult::Cancelled);
        }
        let url = window.url()?;
        if url.path() == RESULT_PATH {
            let raw = url
                .query_pairs()
                .find(|(key, _)| key == "r")
                .map(|(_, value)| value.into_owned())
                .unwrap_or_default();
            let result: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or_default();
            if result["ok"].as_bool() == Some(true) {
                let _ = window.close();
                return Ok(ElybyApplyResult::Done);
            }
            if result["login"].as_bool() == Some(true) {
                window.navigate(ELYBY_LOGIN_URL.parse().expect("url"))?;
                show(&window);
                return Ok(ElybyApplyResult::NeedLogin);
            }
            let page = result["url"]
                .as_str()
                .map(|path| format!("https://ely.by{path}"))
                .unwrap_or_else(|| ELYBY_ADD_URL.to_owned());
            if let Ok(page) = page.parse() {
                window.navigate(page)?;
            }
            show(&window);
            tracing::warn!("Ely.by skin upload: {result}");
            return Ok(ElybyApplyResult::Manual {
                message: result["error"].as_str().unwrap_or("").to_owned(),
            });
        }
        if !injected && url.as_str().starts_with(ELYBY_ADD_URL) {
            // Give the page a moment so `eval` lands in the loaded document.
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            window.eval(upload_script(&texture, slim))?;
            injected = true;
        } else if !injected
            && url.host_str() == Some("ely.by")
            && url.path().starts_with("/authorization")
        {
            show(&window);
            return Ok(ElybyApplyResult::NeedLogin);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    show(&window);
    Ok(ElybyApplyResult::Manual {
        message: "ely.by не ответил за минуту".to_owned(),
    })
}
