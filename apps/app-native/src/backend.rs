//! Связь интерфейса с движком theseus (тот же, что у TrollLauncher на Tauri).
//! Все вызовы идут в tokio, результаты приходят в интерфейс сообщениями `Msg`.

use chrono::{DateTime, Utc};
use eframe::egui;
use serde::Deserialize;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use theseus::prelude::*;
use theseus::data::{ContentType, ResolutionPreferences};

pub const APP_ID: &str = "TrollLauncher";
const UA: &str = "TrollLauncher-native/0.2 (github.com/trollland)";

// ───────────────────────── данные для интерфейса ─────────────────────────

#[derive(Clone, Debug)]
pub struct Inst {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub loader: String,
    pub icon: Option<String>,
    pub last_played: Option<DateTime<Utc>>,
    pub played_secs: u64,
    pub installing: bool,
    pub created: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct ContentRow {
    pub file_path: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub enabled: bool,
    pub has_update: bool,
    pub icon_url: Option<String>,
    pub kind: u8, // 0 мод, 1 ресурспак, 2 шейдер, 3 датапак
    pub project_id: Option<String>,
    pub slug: Option<String>,
    pub version_id: Option<String>,
    pub file_name: String,
    pub size: u64,
    pub published: Option<String>,
    pub license: Option<String>,
    pub categories: Vec<String>,
}

/// Подробности о проекте с Modrinth для окна «О моде».
#[derive(Clone, Debug, Default)]
pub struct ModDetails {
    pub description: String,
    pub body: String,
    pub downloads: u64,
    pub followers: u64,
    pub updated: String,
    pub client_side: String,
    pub server_side: String,
    pub links: Vec<(&'static str, String)>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
    pub changelog: String,
    pub team: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Hit {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub follows: u64,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub display_categories: Vec<String>,
    pub date_modified: String,
    pub project_type: String,
}

#[derive(Clone, Debug)]
pub struct Account {
    pub id: uuid::Uuid,
    pub name: String,
    pub elyby: bool,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub title: String,
    pub progress: Option<f32>,
    pub instance_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SkinItem {
    pub id: String,
    pub name: String,
    pub slim: bool,
    pub favorite: bool,
    pub source: String,
    pub rgba: Option<(u32, u32, Vec<u8>)>,
    pub png: Vec<u8>,
}

pub enum Msg {
    Ready,
    InitError(String),
    Instances(Vec<Inst>),
    Running(HashMap<String, DateTime<Utc>>),
    Content(String, Vec<ContentRow>),
    Search(u64, Result<(Vec<Hit>, u64), String>),
    Users(Vec<Account>, Option<uuid::Uuid>),
    Jobs(Vec<Job>),
    Skins(Vec<SkinItem>, HashMap<String, String>),
    Log(String, String),
    Versions(Vec<String>),
    Toast(String, bool),
    InstallDone(String),
    LoginBusy(bool),
    Picked(Pick, PathBuf),
    Screens(String, Vec<PathBuf>),
    Memory(u32),
    Perf(PerfState),
    ModDetails(String, Result<ModDetails, String>),
    InstSettings(String, crate::instcfg::InstSettings),
    ExportRoot(String, Vec<crate::instcfg::RootEntry>),
    ExportDone(Result<Option<PathBuf>, String>),
    ElyDone,
    Update(Result<Option<crate::update::UpdateInfo>, String>, bool),
    UpdateInstalled(Result<(), String>),
}

/// Настройки производительности (общие с TrollLauncher на Tauri) + поведение окна при запуске игры.
#[derive(Clone, Debug)]
pub struct PerfState {
    pub s: PerformanceSettings,
    pub tools: PerformanceTools,
    /// Свернуть лаунчер, когда игра запустилась (как в Prism)
    pub hide_on_start: bool,
    /// Вернуть окно, когда игра закрылась
    pub refocus_on_close: bool,
}

#[derive(Clone, Debug)]
pub enum Pick {
    InstanceIcon(String),
    NewInstanceIcon,
    SkinFile,
}

// ───────────────────────── backend ─────────────────────────

pub struct Backend {
    rt: tokio::runtime::Runtime,
    tx: Sender<Msg>,
    pub rx: Receiver<Msg>,
    ctx: egui::Context,
    http: reqwest::Client,
}

static ELY_CANCEL: tokio::sync::Notify = tokio::sync::Notify::const_new();

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl Backend {
    pub fn new(ctx: egui::Context) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().worker_threads(4).build().expect("tokio");
        let (tx, rx) = channel();
        let http = reqwest::Client::builder().user_agent(UA).build().expect("http");
        let b = Self { rt, tx, rx, ctx, http };
        b.task(|tx| async move {
            let _ = theseus::EventState::init().await;
            match State::init(APP_ID.to_owned()).await {
                Ok(()) => {
                    let _ = tx.send(Msg::Ready);
                }
                Err(e) => {
                    let _ = tx.send(Msg::InitError(err(e)));
                }
            }
        });
        b
    }

    /// Запустить задачу; по завершении интерфейс перерисуется.
    pub fn task<F, Fut>(&self, f: F)
    where
        F: FnOnce(Sender<Msg>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let ctx = self.ctx.clone();
        let fut = f(self.tx.clone());
        self.rt.spawn(async move {
            fut.await;
            ctx.request_repaint();
        });
    }

    fn toast_err(tx: &Sender<Msg>, what: &str, e: impl std::fmt::Display) {
        let _ = tx.send(Msg::Toast(format!("{what}: {e}"), true));
    }

    // ── сборки

    pub fn refresh_instances(&self) {
        self.task(|tx| async move {
            match instance::list().await {
                Ok(list) => {
                    let mut out: Vec<Inst> = list
                        .into_iter()
                        .map(|m| {
                            let i = m.instance;
                            let cs = m.applied_content_set;
                            Inst {
                                installing: !matches!(i.install_stage.as_str(), "installed" | "pack_installed"),
                                id: i.id,
                                name: i.name,
                                game_version: cs.game_version,
                                loader: loader_name(cs.loader),
                                icon: i.icon_path,
                                last_played: i.last_played,
                                played_secs: i.submitted_time_played + i.recent_time_played,
                                created: i.created,
                            }
                        })
                        .collect();
                    out.sort_by(|a, b| b.last_played.unwrap_or(b.created).cmp(&a.last_played.unwrap_or(a.created)));
                    let _ = tx.send(Msg::Instances(out));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось загрузить сборки", e),
            }
        });
    }

    pub fn poll_status(&self) {
        self.task(|tx| async move {
            if let Ok(ps) = process::get_all().await {
                let map = ps.into_iter().map(|p| (p.instance_id, p.start_time)).collect();
                let _ = tx.send(Msg::Running(map));
            }
            if let Ok(jobs) = install::list_jobs(false).await {
                let jobs = jobs
                    .into_iter()
                    .map(|j| Job {
                        title: j.display.map(|d| d.title).unwrap_or_else(|| "Установка".into()),
                        progress: j.progress.filter(|p| p.total > 0).map(|p| p.current as f32 / p.total as f32),
                        instance_id: j.instance_id,
                    })
                    .collect();
                let _ = tx.send(Msg::Jobs(jobs));
            }
        });
    }

    pub fn run(&self, id: String) {
        self.task(|tx| async move {
            // наш мод sfatm обновляется до запуска; сам движок проверит ещё раз, но уже без загрузки
            if PerformanceSettings::get().await.sfatm_autoupdate {
                match instance::sfatm_update(&id).await {
                    Ok(Some(u)) => {
                        let _ = tx.send(Msg::Toast(format!("sfatm обновлён: {} → {}", u.from, u.to), false));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let _ = tx.send(Msg::Toast(format!("Не удалось обновить sfatm, запускаю старую версию: {e}"), true));
                    }
                }
            }
            if let Err(e) = instance::run(&id, instance::QuickPlayType::None).await {
                Self::toast_err(&tx, "Не удалось запустить", e);
            }
        });
    }

    pub fn kill(&self, id: String) {
        self.task(|tx| async move {
            if let Err(e) = instance::kill(&id).await {
                Self::toast_err(&tx, "Не удалось остановить", e);
            }
        });
    }

    pub fn set_icon(&self, id: String, path: PathBuf) {
        self.task(|tx| async move {
            match instance::edit_icon(&id, Some(&path)).await {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast("Аватарка сборки обновлена".into(), false));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось поставить картинку", e),
            }
        });
        self.refresh_instances();
    }

    pub fn reset_icon(&self, id: String) {
        self.task(|tx| async move {
            if let Err(e) = instance::edit_icon(&id, None).await {
                Self::toast_err(&tx, "Не удалось убрать картинку", e);
            }
        });
        self.refresh_instances();
    }

    pub fn rename(&self, id: String, name: String) {
        self.task(|tx| async move {
            let edit = theseus::data::EditInstance { name: Some(name), ..Default::default() };
            if let Err(e) = instance::edit(&id, edit).await {
                Self::toast_err(&tx, "Не удалось переименовать", e);
            }
        });
        self.refresh_instances();
    }

    pub fn remove_instance(&self, id: String) {
        self.task(|tx| async move {
            match instance::remove(&id).await {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast("Сборка удалена".into(), false));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось удалить", e),
            }
        });
        self.refresh_instances();
    }

    pub fn open_folder(&self, id: String) {
        self.task(|tx| async move {
            match instance::get_full_path(&id).await {
                Ok(p) => {
                    crate::sys::open(p);
                }
                Err(e) => Self::toast_err(&tx, "Папка не найдена", e),
            }
        });
    }

    pub fn create_instance(&self, name: String, version: String, loader: usize, icon: Option<PathBuf>) {
        self.task(|tx| async move {
            let loader = [ModLoader::Vanilla, ModLoader::Fabric, ModLoader::Forge, ModLoader::NeoForge, ModLoader::Quilt][loader];
            let icon = icon.map(|p| p.to_string_lossy().to_string());
            match install::create_instance(name.clone(), version, loader, None, icon, None, theseus::data::InstanceLink::Unmanaged).await {
                Ok(_) => {
                    let _ = tx.send(Msg::Toast(format!("Создаю «{name}»…"), false));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось создать сборку", e),
            }
        });
        self.refresh_instances();
    }

    pub fn load_versions(&self) {
        self.task(|tx| async move {
            if let Ok(m) = metadata::get_minecraft_versions().await {
                let v = m
                    .versions
                    .into_iter()
                    .filter(|v| matches!(v.type_, daedalus::minecraft::VersionType::Release))
                    .map(|v| v.id)
                    .collect();
                let _ = tx.send(Msg::Versions(v));
            }
        });
    }

    // ── контент сборки

    pub fn load_content(&self, id: String) {
        self.task(|tx| async move {
            match instance::get_content_items(&id, None).await {
                Ok(items) => {
                    let mut rows: Vec<ContentRow> = items
                        .into_iter()
                        .map(|c| ContentRow {
                            name: c.project.as_ref().map(|p| p.title.clone()).unwrap_or_else(|| c.file_name.trim_end_matches(".disabled").to_string()),
                            author: c.owner.as_ref().map(|o| o.name.clone()).unwrap_or_default(),
                            version: c.version.as_ref().map(|v| v.version_number.clone()).unwrap_or_default(),
                            icon_url: c.project.as_ref().and_then(|p| p.icon_url.clone()),
                            enabled: c.enabled,
                            has_update: c.has_update,
                            kind: match c.project_type {
                                ProjectType::Mod => 0,
                                ProjectType::ResourcePack => 1,
                                ProjectType::ShaderPack => 2,
                                ProjectType::DataPack => 3,
                            },
                            project_id: c.project.as_ref().map(|p| p.id.clone()),
                            slug: c.project.as_ref().and_then(|p| p.slug.clone()),
                            version_id: c.version.as_ref().map(|v| v.id.clone()),
                            published: c.version.as_ref().and_then(|v| v.date_published.clone()),
                            license: c.project.as_ref().map(|p| p.license.id.clone()).filter(|l| !l.is_empty()),
                            categories: c.project.as_ref().map(|p| p.categories.clone()).unwrap_or_default(),
                            file_name: c.file_name,
                            size: c.size,
                            file_path: c.file_path,
                        })
                        .collect();
                    rows.sort_by_key(|r| r.name.to_lowercase());
                    let _ = tx.send(Msg::Content(id, rows));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось прочитать моды", e),
            }
        });
    }

    pub fn toggle_content(&self, id: String, file: String, enabled: bool) {
        self.task(|tx| async move {
            if let Err(e) = instance::toggle_disable_project(&id, &file, Some(enabled)).await {
                Self::toast_err(&tx, "Не удалось переключить", e);
            }
        });
    }

    pub fn remove_content(&self, id: String, file: String) {
        let id_for_reload = id.clone();
        self.task(|tx| async move {
            match instance::remove_project(&id, &file).await {
                Ok(()) => {}
                Err(e) => Self::toast_err(&tx, "Не удалось удалить", e),
            }
        });
        self.load_content(id_for_reload);
    }

    pub fn update_content(&self, id: String, files: Vec<String>) {
        self.task(|tx| async move {
            let mut ok = 0;
            for f in &files {
                match instance::update_project(&id, f, Some(true)).await {
                    Ok(_) => ok += 1,
                    Err(e) => Self::toast_err(&tx, "Ошибка обновления", e),
                }
            }
            let _ = tx.send(Msg::Toast(format!("Обновлено: {ok}"), false));
            let _ = tx.send(Msg::InstallDone(id));
        });
    }

    // ── Modrinth

    pub fn search(&self, seq: u64, query: String, project_type: &'static str, version: Option<String>, loader: Option<String>, sort: &'static str, offset: u32) {
        let http = self.http.clone();
        self.task(move |tx| async move {
            let mut facets = vec![vec![format!("project_type:{project_type}")]];
            if let Some(v) = version {
                facets.push(vec![format!("versions:{v}")]);
            }
            if let Some(l) = loader
                && (project_type == "mod" || project_type == "modpack")
            {
                facets.push(vec![format!("categories:{l}")]);
            }
            let facets = serde_json::to_string(&facets).unwrap_or_default();
            let res = async {
                let r = http
                    .get("https://api.modrinth.com/v2/search")
                    .query(&[("query", query.as_str()), ("facets", facets.as_str()), ("index", sort), ("limit", "20"), ("offset", &offset.to_string())])
                    .send()
                    .await
                    .map_err(err)?
                    .error_for_status()
                    .map_err(err)?;
                #[derive(Deserialize)]
                struct R {
                    hits: Vec<Hit>,
                    total_hits: u64,
                }
                let r: R = r.json().await.map_err(err)?;
                Ok::<_, String>((r.hits, r.total_hits))
            }
            .await;
            let _ = tx.send(Msg::Search(seq, res));
        });
    }

    /// Поставить проект в сборку (с зависимостями), дождаться появления в списке.
    pub fn install_project(&self, instance_id: String, hit: Hit) {
        self.task(|tx| async move {
            let content_type = match hit.project_type.as_str() {
                "resourcepack" => ContentType::ResourcePack,
                "shader" => ContentType::Shader,
                "datapack" => ContentType::DataPack,
                _ => ContentType::Mod,
            };
            let req = instance::InstallProjectWithDependenciesRequest {
                project_id: hit.project_id.clone(),
                version_id: None,
                content_type,
                selected: ResolutionPreferences::default(),
            };
            match instance::install_project_with_dependencies(&instance_id, req).await {
                Ok(_) => {
                    // установка идёт в фоне: ждём, пока проект появится в сборке
                    for _ in 0..240 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        if let Ok(ids) = instance::get_installed_project_ids(&instance_id).await
                            && ids.contains(&hit.project_id)
                        {
                            let _ = tx.send(Msg::Toast(format!("{} установлен", hit.title), false));
                            let _ = tx.send(Msg::InstallDone(hit.project_id.clone()));
                            return;
                        }
                    }
                    Self::toast_err(&tx, &hit.title, "установка не завершилась за 2 минуты");
                    let _ = tx.send(Msg::InstallDone(hit.project_id));
                }
                Err(e) => {
                    Self::toast_err(&tx, &format!("Не удалось поставить {}", hit.title), e);
                    let _ = tx.send(Msg::InstallDone(hit.project_id));
                }
            }
        });
    }

    pub fn installed_ids(&self, instance_id: String) {
        self.task(|tx| async move {
            if let Ok(ids) = instance::get_installed_project_ids(&instance_id).await {
                for id in ids {
                    let _ = tx.send(Msg::InstallDone(format!("have:{id}")));
                }
            }
        });
    }

    /// Сборка с Modrinth: берём последнюю версию и создаём новую сборку.
    pub fn install_modpack(&self, hit: Hit) {
        let http = self.http.clone();
        self.task(|tx| async move {
            let res = async {
                #[derive(Deserialize)]
                struct V {
                    id: String,
                }
                let vs: Vec<V> = http
                    .get(format!("https://api.modrinth.com/v2/project/{}/version", hit.project_id))
                    .send()
                    .await
                    .map_err(err)?
                    .json()
                    .await
                    .map_err(err)?;
                let v = vs.into_iter().next().ok_or("у сборки нет версий")?;
                install::create_modpack_instance(
                    pack::install_from::CreatePackLocation::FromVersionId {
                        project_id: hit.project_id.clone(),
                        version_id: v.id,
                        title: hit.title.clone(),
                        icon_url: hit.icon_url.clone(),
                    },
                    None,
                )
                .await
                .map_err(err)?;
                Ok::<(), String>(())
            }
            .await;
            match res {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast(format!("Сборка «{}» устанавливается", hit.title), false));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось поставить сборку", e),
            }
            let _ = tx.send(Msg::InstallDone(hit.project_id));
        });
        self.refresh_instances();
    }

    // ── журнал

    pub fn load_log(&self, id: String) {
        self.task(|tx| async move {
            let Ok(dir) = instance::get_full_path(&id).await else { return };
            let text = tokio::fs::read_to_string(dir.join("logs").join("latest.log")).await.unwrap_or_default();
            let tail: Vec<&str> = text.lines().rev().take(400).collect();
            let text = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            let _ = tx.send(Msg::Log(id, text));
        });
    }

    // ── аккаунты

    pub fn load_users(&self) {
        self.task(|tx| async move {
            let users = minecraft_auth::users().await.unwrap_or_default();
            let def = minecraft_auth::get_default_user().await.ok().flatten();
            let list = users
                .into_iter()
                .map(|c| Account {
                    id: c.offline_profile.id,
                    name: c.offline_profile.name.clone(),
                    elyby: serde_json::to_value(&c.provider).ok().and_then(|v| v.as_str().map(|s| s == "elyby")).unwrap_or(false),
                })
                .collect();
            let _ = tx.send(Msg::Users(list, def));
        });
    }

    pub fn set_default_user(&self, id: uuid::Uuid) {
        self.task(|tx| async move {
            if let Err(e) = minecraft_auth::set_default_user(id).await {
                Self::toast_err(&tx, "Не удалось выбрать аккаунт", e);
            }
        });
        self.load_users();
    }

    pub fn remove_user(&self, id: uuid::Uuid) {
        self.task(|tx| async move {
            if let Err(e) = minecraft_auth::remove_user(id).await {
                Self::toast_err(&tx, "Не удалось выйти", e);
            }
        });
        self.load_users();
    }

    pub fn elyby_login(&self, user: String, pass: String, totp: Option<String>) {
        self.task(|tx| async move {
            let _ = tx.send(Msg::LoginBusy(true));
            match minecraft_auth::elyby_login(&user, &pass, totp.as_deref()).await {
                Ok(c) => {
                    let _ = minecraft_auth::set_default_user(c.offline_profile.id).await;
                    let _ = tx.send(Msg::Toast(format!("Вход через Ely.by: {}", c.offline_profile.name), false));
                    let _ = tx.send(Msg::ElyDone);
                }
                Err(e) => Self::toast_err(&tx, "Ely.by", e),
            }
            let _ = tx.send(Msg::LoginBusy(false));
        });
        self.load_users();
    }

    /// Вход Microsoft: окно входа открывается отдельным процессом (WebKit только там).
    pub fn elyby_cancel(&self) {
        ELY_CANCEL.notify_waiters();
    }

    /// Настроен ли вход Ely.by через браузер (есть ключи OAuth-приложения).
    pub fn elyby_oauth_ready() -> bool {
        minecraft_auth::elyby_oauth_client().is_some()
    }

    /// Вход Ely.by через браузер: открываем страницу Ely.by, ловим redirect на localhost.
    pub fn elyby_browser_login(&self) {
        self.task(|tx| async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let _ = tx.send(Msg::LoginBusy(true));
            let res = async {
                let client = minecraft_auth::elyby_oauth_client().ok_or("вход через браузер не настроен")?;
                // настольное приложение Ely.by: любой свободный порт на 127.0.0.1; «веб» — заданный адрес
                let fixed = client.redirect_uri.as_deref().and_then(|u| url::Url::parse(u).ok());
                let port = fixed.as_ref().and_then(|u| u.port()).unwrap_or(0);
                let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.map_err(|e| format!("порт {port} занят: {e}"))?;
                let redirect_uri = match &fixed {
                    Some(u) => u.to_string(),
                    None => format!("http://127.0.0.1:{}/ely", listener.local_addr().map_err(err)?.port()),
                };
                let redirect = url::Url::parse(&redirect_uri).map_err(err)?;
                let start = minecraft_auth::elyby_oauth_start(&client, &redirect_uri);
                let state = start.state.clone();
                crate::sys::open(&start.url);
                let wait = async {
                    loop {
                        let (mut sock, _) = listener.accept().await.map_err(err)?;
                        let mut buf = vec![0u8; 8192];
                        let n = sock.read(&mut buf).await.unwrap_or(0);
                        let req = String::from_utf8_lossy(&buf[..n]).to_string();
                        let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                        let Ok(u) = url::Url::parse(&format!("http://localhost{path}")) else { continue };
                        if u.path() != redirect.path() {
                            let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n").await;
                            continue;
                        }
                        let q: HashMap<String, String> = u.query_pairs().into_owned().collect();
                        let ok = q.get("state") == Some(&state) && q.contains_key("code");
                        let (title, text) = if ok {
                            ("Готово", "Вход через Ely.by выполнен. Можно закрыть вкладку и вернуться в TrollLauncher.")
                        } else {
                            ("Вход отменён", "Ely.by не подтвердил вход. Вернитесь в TrollLauncher и попробуйте ещё раз.")
                        };
                        let body = format!(
                            "<!doctype html><meta charset=utf-8><title>TrollLauncher</title><body style=\"background:#06060b;color:#e8e8f0;font:16px system-ui;display:grid;place-items:center;height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"color:#5b58ff\">{title}</h1><p>{text}</p></div><script>setTimeout(()=>window.close(),1500)</script>"
                        );
                        let _ = sock
                            .write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes())
                            .await;
                        return if ok { Ok(q["code"].clone()) } else { Err(q.get("error_description").or(q.get("error")).cloned().unwrap_or_else(|| "вход отменён".into())) };
                    }
                };
                let code = tokio::select! {
                    r = tokio::time::timeout(std::time::Duration::from_secs(300), wait) => r.map_err(|_| "время ожидания вышло".to_string())??,
                    _ = ELY_CANCEL.notified() => return Err("вход отменён".to_string()),
                };
                minecraft_auth::elyby_login_oauth(&code, &start).await.map_err(err)
            }
            .await;
            match res {
                Ok(c) => {
                    let _ = tx.send(Msg::Toast(format!("Вход через Ely.by: {}", c.offline_profile.name), false));
                    let _ = tx.send(Msg::ElyDone);
                }
                Err(e) => Self::toast_err(&tx, "Ely.by", e),
            }
            let _ = tx.send(Msg::LoginBusy(false));
        });
        self.load_users();
    }

    pub fn microsoft_login(&self) {
        let ctx = self.ctx.clone();
        self.task(|tx| async move {
            let _ = tx.send(Msg::LoginBusy(true));
            let res = async {
                let flow = minecraft_auth::begin_login().await.map_err(err)?;
                let exe = std::env::current_exe().map_err(err)?;
                let out = tokio::process::Command::new(exe).arg("--msa-login").arg(&flow.auth_request_uri).output().await.map_err(err)?;
                let code = String::from_utf8_lossy(&out.stdout).lines().find_map(|l| l.strip_prefix("CODE=").map(str::to_string));
                let Some(code) = code else { return Ok(None) };
                let c = minecraft_auth::finish_login(&code, flow).await.map_err(err)?;
                let _ = minecraft_auth::set_default_user(c.offline_profile.id).await;
                Ok::<_, String>(Some(c.offline_profile.name.clone()))
            }
            .await;
            match res {
                Ok(Some(name)) => {
                    let _ = tx.send(Msg::Toast(format!("Вход выполнен: {name}"), false));
                }
                Ok(None) => {
                    let _ = tx.send(Msg::Toast("Вход отменён".into(), false));
                }
                Err(e) => Self::toast_err(&tx, "Microsoft", e),
            }
            let _ = tx.send(Msg::LoginBusy(false));
            ctx.request_repaint();
        });
    }

    // ── скины

    pub fn load_skins(&self) {
        self.task(|tx| async move {
            let lib = match theseus::skin_library::get().await {
                Ok(l) => l,
                Err(e) => return Self::toast_err(&tx, "Библиотека скинов", e),
            };
            let mut items = vec![];
            for s in lib.skins {
                let png = theseus::skin_library::texture(&s.id).await.map(|b| b.to_vec()).unwrap_or_default();
                let rgba = image::load_from_memory(&png).ok().map(|i| {
                    let i = i.to_rgba8();
                    (i.width(), i.height(), i.into_raw())
                });
                items.push(SkinItem {
                    id: s.id,
                    name: s.name,
                    slim: matches!(s.variant, theseus::skin_library::LibrarySkinVariant::Slim),
                    favorite: s.favorite,
                    source: s.source,
                    rgba,
                    png,
                });
            }
            let _ = tx.send(Msg::Skins(items, lib.equipped));
        });
    }

    pub fn add_skin_file(&self, path: PathBuf) {
        self.task(|tx| async move {
            let res = async {
                let bytes = tokio::fs::read(&path).await.map_err(err)?;
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Скин".into());
                theseus::skin_library::add(bytes, name, None, None, "file".into()).await.map_err(err)?;
                Ok::<(), String>(())
            }
            .await;
            match res {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast("Скин добавлен".into(), false));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось добавить", e),
            }
        });
        self.load_skins();
    }

    pub fn add_skin_nick(&self, nick: String, elyby: bool) {
        self.task(|tx| async move {
            let src = if elyby { "elyby" } else { "mojang" };
            let res = async {
                let f = theseus::skin_library::fetch_by_nick(&nick, src).await.map_err(err)?;
                theseus::skin_library::add(f.texture.to_vec(), f.name, Some(f.variant), None, src.into()).await.map_err(err)?;
                Ok::<(), String>(())
            }
            .await;
            match res {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast(format!("Скин {nick} добавлен"), false));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось найти скин", e),
            }
        });
        self.load_skins();
    }

    pub fn update_skin(&self, id: String, name: Option<String>, favorite: Option<bool>, slim: Option<bool>) {
        self.task(|tx| async move {
            let res = async {
                let lib = theseus::skin_library::get().await.map_err(err)?;
                let mut s = lib.skins.into_iter().find(|s| s.id == id).ok_or("скин не найден")?;
                if let Some(n) = name {
                    s.name = n;
                }
                if let Some(f) = favorite {
                    s.favorite = f;
                }
                if let Some(sl) = slim {
                    s.variant = if sl { theseus::skin_library::LibrarySkinVariant::Slim } else { theseus::skin_library::LibrarySkinVariant::Classic };
                }
                theseus::skin_library::update(s).await.map_err(err)
            }
            .await;
            if let Err(e) = res {
                Self::toast_err(&tx, "Скин", e);
            }
        });
        self.load_skins();
    }

    pub fn remove_skin(&self, id: String) {
        self.task(|tx| async move {
            if let Err(e) = theseus::skin_library::remove(&id).await {
                Self::toast_err(&tx, "Не удалось удалить скин", e);
            }
        });
        self.load_skins();
    }

    pub fn equip_skin(&self, account: Account, skin: SkinItem) {
        self.task(|tx| async move {
            if account.elyby {
                let _ = tx.send(Msg::Toast("Для Ely.by скин меняется на сайте ely.by — открываю страницу".into(), false));
                crate::sys::open("https://ely.by/skins");
                return;
            }
            let variant = if skin.slim { theseus::minecraft_skins::MinecraftSkinVariant::Slim } else { theseus::minecraft_skins::MinecraftSkinVariant::Classic };
            let res = async {
                theseus::minecraft_skins::add_and_equip_custom_skin(bytes::Bytes::from(skin.png.clone()), variant, None).await.map_err(err)?;
                theseus::minecraft_skins::flush_pending_skin_change().await.map_err(err)?;
                theseus::skin_library::set_equipped(account.id.to_string(), skin.id.clone()).await.map_err(err)?;
                Ok::<(), String>(())
            }
            .await;
            match res {
                Ok(()) => {
                    let _ = tx.send(Msg::Toast(format!("Скин «{}» надет", skin.name), false));
                }
                Err(e) => Self::toast_err(&tx, "Не удалось надеть скин", e),
            }
        });
        self.load_skins();
    }

    // ── файлы

    pub fn pick(&self, what: Pick) {
        self.task(|tx| async move {
            let dlg = rfd::AsyncFileDialog::new();
            let dlg = match what {
                Pick::SkinFile => dlg.set_title("Выберите скин (PNG 64×64)").add_filter("PNG", &["png"]),
                _ => dlg.set_title("Выберите картинку").add_filter("Картинки", &["png", "jpg", "jpeg", "webp", "gif"]),
            };
            if let Some(f) = dlg.pick_file().await {
                let _ = tx.send(Msg::Picked(what, f.path().to_path_buf()));
            }
        });
    }

    pub fn load_screens(&self, id: String) {
        self.task(|tx| async move {
            let Ok(dir) = instance::get_full_path(&id).await else { return };
            let mut out = vec![];
            if let Ok(mut rd) = tokio::fs::read_dir(dir.join("screenshots")).await {
                while let Ok(Some(e)) = rd.next_entry().await {
                    let p = e.path();
                    if p.extension().is_some_and(|x| x == "png" || x == "jpg") {
                        out.push(p);
                    }
                }
            }
            out.sort();
            out.reverse();
            let _ = tx.send(Msg::Screens(id, out));
        });
    }

    // ── настройки

    pub fn get_memory(&self) {
        self.task(|tx| async move {
            if let Ok(s) = settings::get().await {
                let _ = tx.send(Msg::Memory(s.memory.maximum));
            }
        });
    }

    pub fn set_memory(&self, mb: u32) {
        self.task(|tx| async move {
            match settings::get().await {
                Ok(mut s) => {
                    s.memory.maximum = mb;
                    if let Err(e) = settings::set(s).await {
                        Self::toast_err(&tx, "Настройки", e);
                    }
                }
                Err(e) => Self::toast_err(&tx, "Настройки", e),
            }
        });
    }

    pub fn load_perf(&self) {
        self.task(|tx| async move {
            let s = PerformanceSettings::get().await;
            let tools = performance::available_tools();
            let (hide, refocus) = settings::get().await.map(|st| (st.hide_on_process_start, st.refocus_on_game_close)).unwrap_or((false, false));
            let _ = tx.send(Msg::Perf(PerfState { s, tools, hide_on_start: hide, refocus_on_close: refocus }));
        });
    }

    pub fn save_perf(&self, p: PerfState) {
        self.task(|tx| async move {
            if let Err(e) = p.s.set().await {
                Self::toast_err(&tx, "Производительность", e);
            }
            match settings::get().await {
                Ok(mut st) if st.hide_on_process_start != p.hide_on_start || st.refocus_on_game_close != p.refocus_on_close => {
                    st.hide_on_process_start = p.hide_on_start;
                    st.refocus_on_game_close = p.refocus_on_close;
                    if let Err(e) = settings::set(st).await {
                        Self::toast_err(&tx, "Настройки", e);
                    }
                }
                Ok(_) => {}
                Err(e) => Self::toast_err(&tx, "Настройки", e),
            }
        });
    }

    pub fn reveal_content(&self, id: String, file_path: String) {
        self.task(|_tx| async move {
            let Ok(dir) = instance::get_full_path(&id).await else { return };
            let p = dir.join(&file_path);
            let target = p.parent().map(|x| x.to_path_buf()).unwrap_or(dir);
            crate::sys::open(target);
        });
    }

    // ── окно «О моде»

    pub fn load_mod_details(&self, project_id: String, version_id: Option<String>) {
        let http = self.http.clone();
        self.task(|tx| async move {
            let res = async {
                let get = |url: String| {
                    let http = http.clone();
                    async move { http.get(url).send().await.map_err(err)?.error_for_status().map_err(err)?.json::<serde_json::Value>().await.map_err(err) }
                };
                let p = get(format!("https://api.modrinth.com/v2/project/{project_id}")).await?;
                let st = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let arr = |k: &str| p.get(k).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                let mut links = vec![];
                for (k, name) in [("source_url", "Исходный код"), ("issues_url", "Баг-трекер"), ("wiki_url", "Вики"), ("discord_url", "Discord")] {
                    let u = st(k);
                    if !u.is_empty() {
                        links.push((name, u));
                    }
                }
                let changelog = match version_id {
                    Some(v) => get(format!("https://api.modrinth.com/v2/version/{v}")).await.ok().and_then(|v| v.get("changelog").and_then(|c| c.as_str()).map(String::from)).unwrap_or_default(),
                    None => String::new(),
                };
                let team = get(format!("https://api.modrinth.com/v2/project/{project_id}/members"))
                    .await
                    .ok()
                    .and_then(|v| v.as_array().cloned())
                    .map(|a| a.iter().filter_map(|m| m.pointer("/user/username").and_then(|x| x.as_str()).map(String::from)).collect())
                    .unwrap_or_default();
                Ok::<_, String>(ModDetails {
                    description: st("description"),
                    body: st("body"),
                    downloads: p.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                    followers: p.get("followers").and_then(|v| v.as_u64()).unwrap_or(0),
                    updated: st("updated"),
                    client_side: st("client_side"),
                    server_side: st("server_side"),
                    links,
                    loaders: arr("loaders"),
                    game_versions: arr("game_versions"),
                    changelog,
                    team,
                })
            }
            .await;
            let _ = tx.send(Msg::ModDetails(project_id, res));
        });
    }

    // ── обновление лаунчера

    /// `manual` — нажали «Проверить», тогда сообщаем и «обновлений нет».
    pub fn check_update(&self, manual: bool) {
        self.task(move |tx| async move {
            let _ = tx.send(Msg::Update(crate::update::check().await, manual));
        });
    }

    pub fn install_update(&self, info: crate::update::UpdateInfo) {
        self.task(move |tx| async move {
            let _ = tx.send(Msg::UpdateInstalled(crate::update::install(&info).await));
        });
    }
}

pub fn loader_name(l: ModLoader) -> String {
    match l {
        ModLoader::Vanilla => "Vanilla",
        ModLoader::Forge => "Forge",
        ModLoader::Fabric => "Fabric",
        ModLoader::Quilt => "Quilt",
        ModLoader::NeoForge => "NeoForge",
    }
    .into()
}
