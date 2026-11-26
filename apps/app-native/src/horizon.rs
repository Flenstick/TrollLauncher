//! Сборки Horizon — Minecraft, переписанный на Rust (horizon-launcher). Лаунчер вызывает его
//! командную строку: `instance list/create/delete/version`, `versions`, `install`, `play --instance`.
//! Если Horizon Launcher не установлен, он скачивается с релизов GitHub Horizon в `<данные>/horizon/`.
//! Сборки Horizon показываются рядом с обычными с id `horizon:<id>`.

use crate::backend::{Backend, Inst, Msg};
use chrono::{DateTime, Utc};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub const PREFIX: &str = "horizon:";
const REPO: &str = "horizon-rust/horizon";
/// Последняя версия Horizon (а не закреплённая)
pub const LATEST: &str = "latest";

pub fn is_horizon(id: &str) -> bool {
    id.starts_with(PREFIX)
}

fn raw_id(id: &str) -> &str {
    id.strip_prefix(PREFIX).unwrap_or(id)
}

const EXE: &str = if cfg!(windows) { "horizon-launcher.exe" } else { "horizon-launcher" };
const ASSET: &str = if cfg!(windows) { "horizon-launcher-windows-x64.zip" } else { "horizon-launcher-linux-x86_64.tar.gz" };

/// Своя копия, скачанная лаунчером.
fn own_bin() -> PathBuf {
    crate::prefs::data_dir().join("horizon").join(EXE)
}

/// Где лежит horizon-launcher: переменная HORIZON_LAUNCHER, своя копия, PATH, ~/.local/bin, ~/Downloads/horizon-launcher-*/.
pub fn bin() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HORIZON_LAUNCHER").map(PathBuf::from)
        && p.is_file()
    {
        return Some(p);
    }
    let own = own_bin();
    if own.is_file() {
        return Some(own);
    }
    if let Some(path) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&path) {
            let p = d.join(EXE);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let home = dirs::home_dir()?;
    let mut found: Vec<PathBuf> = [home.join(".local/bin")].into_iter().map(|d| d.join(EXE)).filter(|p| p.is_file()).collect();
    for dl in ["Downloads", "Загрузки"] {
        for e in std::fs::read_dir(home.join(dl)).into_iter().flatten().flatten() {
            if e.file_name().to_string_lossy().starts_with("horizon-launcher") {
                let p = e.path().join(EXE);
                if p.is_file() {
                    found.push(p);
                }
            }
        }
    }
    found.sort();
    found.pop()
}

#[derive(serde::Deserialize)]
struct Listed {
    id: String,
    name: String,
    #[serde(default)]
    home: String,
    #[serde(default)]
    java: Option<serde_json::Value>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    played_secs: u64,
    #[serde(default)]
    last_played_ms: Option<i64>,
}

/// Сборка Horizon для интерфейса: сама сборка, её папка и закреплённая версия (None — последняя).
pub type HorizonItem = (Inst, String, Option<String>);

fn run(args: &[&str]) -> Result<String, String> {
    let b = bin().ok_or("Horizon Launcher не установлен")?;
    let out = Command::new(b).args(args).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        let e = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(nice_error(if e.is_empty() { format!("код {}", out.status) } else { e }))
    }
}

/// GitHub Horizon отвечает 404, пока репозиторий закрыт — объясняем это по-человечески.
fn nice_error(e: String) -> String {
    if e.contains("404") && e.contains("github") {
        "сервер Horizon (GitHub horizon-rust/horizon) сейчас недоступен — скачать версии нельзя, уже установленные работают".into()
    } else {
        e
    }
}

/// Сборки Horizon на Rust (Java-сборки Horizon не показываем — такие удобнее делать здесь обычными).
fn list() -> Vec<HorizonItem> {
    let Ok(s) = run(&["instance", "list", "--json"]) else { return vec![] };
    let Ok(items) = serde_json::from_str::<Vec<Listed>>(&s) else { return vec![] };
    items
        .into_iter()
        .filter(|i| i.java.as_ref().is_none_or(|j| j.is_null()))
        .map(|i| {
            let last = i.last_played_ms.and_then(DateTime::<Utc>::from_timestamp_millis);
            (
                Inst {
                    id: format!("{PREFIX}{}", i.id),
                    name: i.name,
                    game_version: "Horizon".into(),
                    loader: "Rust".into(),
                    icon: None,
                    last_played: last,
                    played_secs: i.played_secs,
                    installing: false,
                    created: last.unwrap_or_default(),
                    groups: vec![],
                },
                i.home,
                i.version,
            )
        })
        .collect()
}

/// Теги версий из JSON (формат команды может меняться — берём все строки, похожие на теги).
fn tags(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) if s.starts_with('v') && s[1..].starts_with(|c: char| c.is_ascii_digit()) || s.starts_with("nightly-") => {
            if !out.contains(s) {
                out.push(s.clone());
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| tags(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| tags(x, out)),
        _ => {}
    }
}

/// Версии: (установленные, доступные для скачивания или ошибка).
fn versions() -> (Vec<String>, Result<Vec<String>, String>) {
    let mut installed = vec![];
    if let Ok(s) = run(&["versions", "--installed", "--json"])
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&s)
    {
        tags(v.get("installed").unwrap_or(&v), &mut installed);
    }
    let remote = run(&["versions", "--json"]).and_then(|s| {
        let v: serde_json::Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
        let mut out = vec![];
        tags(&v, &mut out);
        Ok(out)
    });
    (installed, remote)
}

/// Поставить версию, если её ещё нет, и закрепить её за сборкой (`latest` — следовать за обновлениями).
fn use_version(instance: &str, tag: &str) -> Result<(), String> {
    if tag != LATEST {
        let (installed, _) = versions();
        if !installed.iter().any(|t| t == tag) {
            run(&["install", tag])?;
        }
    } else if versions().0.is_empty() {
        run(&["install", LATEST])?;
    }
    run(&["instance", "version", instance, tag]).map(|_| ())
}

/// Скачать Horizon Launcher с релизов GitHub Horizon в `<данные>/horizon/`.
async fn download_launcher(http: &reqwest::Client) -> Result<PathBuf, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=40");
    let r = http.get(&url).send().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("сервер Horizon (GitHub {REPO}) сейчас недоступен, ответ {}. Horizon Launcher можно поставить вручную с playhorizon.dev — TrollLauncher найдёт его сам", r.status().as_u16()));
    }
    let releases: serde_json::Value = r.json().await.map_err(|e| e.to_string())?;
    let asset = releases
        .as_array()
        .into_iter()
        .flatten()
        .filter(|rel| !rel.get("draft").and_then(|d| d.as_bool()).unwrap_or(false))
        .find_map(|rel| {
            let assets = rel.get("assets")?.as_array()?;
            let file = assets.iter().find(|a| a.get("name").and_then(|n| n.as_str()) == Some(ASSET))?;
            let sums = assets.iter().find(|a| a.get("name").and_then(|n| n.as_str()) == Some("SHA256SUMS"));
            let url = |a: &serde_json::Value| a.get("browser_download_url").and_then(|u| u.as_str()).map(String::from);
            Some((url(file)?, sums.and_then(url)))
        })
        .ok_or("в релизах Horizon нет сборки лаунчера для этой системы")?;
    let bytes = http.get(&asset.0).send().await.and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
    if let Some(sums_url) = asset.1
        && let Ok(sums) = http.get(&sums_url).send().await.and_then(|r| r.error_for_status())
        && let Ok(text) = sums.text().await
        && let Some(want) = text.lines().find(|l| l.trim_end().ends_with(ASSET)).and_then(|l| l.split_whitespace().next())
    {
        use sha2::Digest;
        let got = format!("{:x}", sha2::Sha256::digest(&bytes));
        if !got.eq_ignore_ascii_case(want) {
            return Err("контрольная сумма Horizon Launcher не совпала — файл не поставлен".into());
        }
    }
    let exe = extract(&bytes)?;
    let dest = own_bin();
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, exe).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    }
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Достать сам исполняемый файл из архива релиза.
fn extract(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if cfg!(windows) {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| e.to_string())?;
            if f.name().rsplit('/').next() == Some(EXE) {
                let mut out = vec![];
                f.read_to_end(&mut out).map_err(|e| e.to_string())?;
                return Ok(out);
            }
        }
    } else {
        let mut t = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
        for e in t.entries().map_err(|e| e.to_string())? {
            let mut e = e.map_err(|e| e.to_string())?;
            if e.path().ok().and_then(|p| p.file_name().map(|n| n == EXE)).unwrap_or(false) {
                let mut out = vec![];
                e.read_to_end(&mut out).map_err(|e| e.to_string())?;
                return Ok(out);
            }
        }
    }
    Err("в архиве Horizon нет horizon-launcher".into())
}

impl Backend {
    pub fn horizon_refresh(&self) {
        self.task(|tx| async move {
            let v = tokio::task::spawn_blocking(list).await.unwrap_or_default();
            let _ = tx.send(Msg::Horizon(v));
        });
    }

    /// Список версий Horizon (установленные и доступные).
    pub fn horizon_versions(&self) {
        self.task(|tx| async move {
            let (installed, remote) = tokio::task::spawn_blocking(versions).await.unwrap_or_else(|e| (vec![], Err(e.to_string())));
            let _ = tx.send(Msg::HorizonVersions(installed, remote));
        });
    }

    /// Скачать Horizon Launcher (если его нет) и прочитать версии.
    pub fn horizon_get_launcher(&self) {
        let http = self.http.clone();
        self.task(|tx| async move {
            let _ = tx.send(Msg::HorizonBusy(Some("Скачиваю Horizon Launcher…".into())));
            match download_launcher(&http).await {
                Ok(_) => {
                    let _ = tx.send(Msg::Toast("Horizon Launcher установлен".into(), false));
                    let (installed, remote) = tokio::task::spawn_blocking(versions).await.unwrap_or_else(|e| (vec![], Err(e.to_string())));
                    let _ = tx.send(Msg::HorizonVersions(installed, remote));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Toast(format!("Horizon: {e}"), true));
                }
            }
            let _ = tx.send(Msg::HorizonBusy(None));
        });
    }

    /// Создать сборку Horizon на выбранной версии (версия скачивается, если её нет).
    pub fn horizon_create(&self, name: String, tag: String) {
        self.task(|tx| async move {
            let _ = tx.send(Msg::HorizonBusy(Some(format!("Создаю «{name}»…"))));
            let res = tokio::task::spawn_blocking(move || {
                run(&["instance", "create", &name])?;
                use_version(&name, &tag)
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
            match res {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast("Сборка Horizon создана".into(), false));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Toast(format!("Horizon: {e}"), true));
                }
            }
            let _ = tx.send(Msg::HorizonBusy(None));
            let v = tokio::task::spawn_blocking(list).await.unwrap_or_default();
            let _ = tx.send(Msg::Horizon(v));
        });
    }

    /// Сменить версию сборки Horizon (скачивается, если её нет).
    pub fn horizon_set_version(&self, id: String, tag: String) {
        self.task(|tx| async move {
            let _ = tx.send(Msg::HorizonBusy(Some(format!("Ставлю версию {tag}…"))));
            let rid = raw_id(&id).to_string();
            let t = tag.clone();
            match tokio::task::spawn_blocking(move || use_version(&rid, &t)).await.unwrap_or_else(|e| Err(e.to_string())) {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast(if tag == LATEST { "Сборка будет на последней версии Horizon".into() } else { format!("Сборка на версии {tag}") }, false));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Toast(format!("Horizon: {e}"), true));
                }
            }
            let _ = tx.send(Msg::HorizonBusy(None));
            let (installed, remote) = tokio::task::spawn_blocking(versions).await.unwrap_or_else(|e| (vec![], Err(e.to_string())));
            let _ = tx.send(Msg::HorizonVersions(installed, remote));
            let v = tokio::task::spawn_blocking(list).await.unwrap_or_default();
            let _ = tx.send(Msg::Horizon(v));
        });
    }

    pub fn horizon_delete(&self, id: String) {
        self.task(|tx| async move {
            let rid = raw_id(&id).to_string();
            match tokio::task::spawn_blocking(move || run(&["instance", "delete", &rid])).await.unwrap_or_else(|e| Err(e.to_string())) {
                Ok(_) => {
                    let _ = tx.send(Msg::Toast("Сборка Horizon убрана в корзину".into(), false));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Toast(format!("Horizon: {e}"), true));
                }
            }
            let v = tokio::task::spawn_blocking(list).await.unwrap_or_default();
            let _ = tx.send(Msg::Horizon(v));
        });
    }
}

/// Запустить сборку Horizon (игра — отдельный процесс horizon-launcher).
pub fn play(id: &str) -> Result<Child, String> {
    let b = bin().ok_or("Horizon Launcher не установлен")?;
    Command::new(b).args(["play", "--instance", raw_id(id)]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())
}

// ───────────────────────── окно ─────────────────────────

use crate::app::App;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui;

impl App {
    /// Выбор версии Horizon: последняя, установленные и доступные для скачивания.
    pub fn horizon_version_combo(&mut self, ui: &mut egui::Ui, salt: &str, tag: &mut String) -> bool {
        let shown = |t: &str| if t == LATEST { "Последняя (обновляется сама)".to_string() } else { t.to_string() };
        let mut changed = false;
        let remote: Vec<String> = match &self.horizon_remote {
            Some(Ok(v)) => v.iter().filter(|t| !self.horizon_installed.contains(t)).cloned().collect(),
            _ => vec![],
        };
        egui::ComboBox::from_id_salt(salt).selected_text(shown(tag)).width(260.0).height(320.0).show_ui(ui, |ui| {
            changed |= ui.selectable_value(tag, LATEST.to_string(), shown(LATEST)).changed();
            for t in &self.horizon_installed {
                changed |= ui.selectable_value(tag, t.clone(), format!("{t}  ✓")).on_hover_text("Установлена").changed();
            }
            for t in &remote {
                changed |= ui.selectable_value(tag, t.clone(), format!("{t}  ⬇")).on_hover_text("Скачается при выборе").changed();
            }
        });
        match &self.horizon_remote {
            None => {
                label(ui, "Ищу версии…", regular(12.5), TEXT_FAINT());
            }
            Some(Err(e)) => {
                ui.add(egui::Label::new(egui::RichText::new(format!("Новые версии не загрузить: {e}")).font(regular(12.5)).color(TEXT_FAINT())).wrap());
            }
            _ => {}
        }
        changed
    }

    /// Окно сборки Horizon: версия, играть, папка, удалить.
    pub fn horizon_dialog(&mut self, ctx: &egui::Context) {
        let Some(id) = self.horizon_open.clone() else { return };
        let Some(inst) = self.inst(&id).cloned() else {
            self.horizon_open = None;
            return;
        };
        let frame = egui::Frame::new().fill(PANEL()).corner_radius(rr(14)).stroke(egui::Stroke::new(1.0, BORDER())).inner_margin(egui::Margin::same(22));
        let mut close = false;
        let r = egui::Modal::new(egui::Id::new("horizon-inst")).frame(frame).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.spacing_mut().item_spacing.y = 10.0;
            label(ui, &inst.name, bold(20.0), TEXT());
            label(ui, format!("Horizon — Minecraft на Rust · в игре {}", crate::app::fmt_played(inst.played_secs)), regular(13.0), TEXT_DIM());
            label(ui, "Версия Horizon", semibold(13.0), TEXT_DIM());
            let mut tag = self.horizon_ver.get(&id).cloned().flatten().unwrap_or_else(|| LATEST.into());
            ui.add_enabled_ui(self.horizon_busy.is_none(), |ui| {
                if self.horizon_version_combo(ui, "hz-inst-ver", &mut tag) {
                    self.be.horizon_set_version(id.clone(), tag.clone());
                }
            });
            if let Some(b) = &self.horizon_busy {
                label(ui, b, medium(13.0), accent());
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
            label(ui, "Моды и паки Horizon ставятся в самом Horizon Launcher.", regular(12.5), TEXT_FAINT());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let running = self.is_running(&id);
                if button(ui, if running { "Остановить" } else { "Играть" }, Some(if running { Icon::Stop } else { Icon::Play }), if running { Kind::Danger } else { Kind::Primary }, 36.0).clicked() {
                    self.play(ui, &id);
                }
                if button(ui, "Папка", Some(Icon::Folder), Kind::Secondary, 36.0).clicked()
                    && let Some(h) = self.horizon_home.get(&id)
                {
                    crate::sys::open(h);
                }
                if button(ui, "Удалить", None, Kind::Secondary, 36.0).clicked() {
                    self.confirm_delete = Some(id.clone());
                    close = true;
                }
                if button(ui, "Закрыть", None, Kind::Ghost, 36.0).clicked() {
                    close = true;
                }
            });
        });
        if close || r.should_close() {
            self.horizon_open = None;
        }
    }
}
