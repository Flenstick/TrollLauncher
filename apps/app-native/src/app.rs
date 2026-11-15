//! Каркас окна: боковая панель, верхняя строка, страницы, диалоги, уведомления.
//! Данные приходят из движка theseus через `Backend`.

use crate::backend::*;
use crate::theme::*;
use crate::widgets::*;
use chrono::{DateTime, Utc};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Frame, Id, Layout, Margin, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui,
    Vec2, ViewportCommand, pos2, vec2,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug)]
pub enum Page {
    Home,
    Library,
    Browse,
    Skins,
    Settings,
    Instance(String),
}

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub born: f64,
}

pub struct Settings {
    pub ram: f32,
}

#[derive(Default)]
pub struct CreateDialog {
    pub open: bool,
    pub name: String,
    pub version: usize,
    pub loader: usize,
    pub icon: Option<PathBuf>,
    pub snapshots: bool,
}

#[derive(Default)]
pub struct ElyDialog {
    pub open: bool,
    pub user: String,
    pub pass: String,
    pub totp: String,
    pub show_pass: bool,
}

#[derive(Default)]
pub struct NickDialog {
    pub open: bool,
    pub nick: String,
    pub source: usize,
}

#[derive(Default)]
pub struct RenameDialog {
    pub open: bool,
    pub id: String,
    pub name: String,
}

pub struct SkinTex {
    pub tex: TextureHandle,
    pub h: f32,
}

/// Номер раздела «Аккаунты» в настройках (для переходов с других страниц).
pub const ACCOUNTS_SECTION: usize = 4;

pub struct App {
    pub be: Backend,
    pub ready: bool,
    pub init_error: Option<String>,

    pub page: Page,
    pub back: Vec<Page>,
    pub fwd: Vec<Page>,
    pub page_changed: f64,

    pub instances: Vec<Inst>,
    pub running: HashMap<String, DateTime<Utc>>,
    pub launching: HashMap<String, f64>,
    pub jobs: Vec<Job>,
    pub content: HashMap<String, Vec<ContentRow>>,
    pub logs: HashMap<String, String>,
    pub screens: HashMap<String, Vec<PathBuf>>,
    pub accounts: Vec<Account>,
    pub default_user: Option<uuid::Uuid>,
    pub login_busy: bool,
    pub versions: Vec<String>,

    pub lib_filter: usize,
    pub lib_search: String,
    pub lib_sort: usize,

    pub browse_tab: usize,
    pub browse_query: String,
    pub browse_sort: usize,
    pub browse_target: Option<String>,
    pub browse_match: bool,
    pub browse_sig: String,
    pub browse_edit_at: f64,
    pub browse_seq: u64,
    pub browse_offset: u32,
    pub browse_loading: bool,
    pub hits: Vec<Hit>,
    pub hits_total: u64,
    pub browse_error: Option<String>,
    pub installing: HashSet<String>,
    pub installed: HashSet<String>,

    pub inst_tab: usize,
    pub mod_search: String,
    pub mod_filter: usize,

    pub skins: Vec<SkinItem>,
    pub skin_tex: HashMap<String, SkinTex>,
    pub skin_equipped: HashMap<String, String>,
    pub skin_sel: Option<String>,
    pub skin_tab: usize,
    pub skin_yaw: f32,
    pub skin_walk: bool,

    pub settings_section: usize,
    pub s: Settings,
    pub prefs: crate::prefs::Prefs,
    pub cats: Vec<crate::cats::CatPack>,
    pub cat_seed: u64,

    pub create: CreateDialog,
	pub import_open: bool,
	pub dropped_mods: Vec<PathBuf>,
	pub drop_target: Option<String>,
    pub ely: ElyDialog,
    pub nick: NickDialog,
    pub rename: RenameDialog,
    pub confirm_delete: Option<String>,

    pub toasts: Vec<Toast>,
    pub perf: Option<PerfState>,
    pub mod_info: Option<crate::modinfo::ModInfoView>,
    pub mod_details: HashMap<String, Result<ModDetails, String>>,
    pub md_cache: HashMap<String, Vec<crate::modinfo::Blk>>,
    pub inst_settings: Option<crate::instcfg::SettingsDialog>,
    pub export: Option<crate::instcfg::ExportDialog>,
    pub update: Option<crate::update::UpdateInfo>,
    pub project_view: Option<crate::modinfo::ProjectView>,
    pub proj_versions: HashMap<String, Vec<crate::backend::ProjVersion>>,
    pub categories: HashMap<String, Vec<crate::backend::Category>>,
    pub browse_cats: std::collections::BTreeSet<String>,
    /// 0 — Modrinth, 1 — CurseForge
    pub browse_source: usize,
    pub cf_key_edit: String,
    pub java: Option<(Vec<(u32, String, String)>, Vec<(u32, String, String)>)>,
    pub launcher_dir: Option<String>,
    pub dir_pending: Option<PathBuf>,
    skins_checked_for: Vec<uuid::Uuid>,
    /// 0 — нет, 1 — ставится, 2 — поставлено (ждёт перезапуска)
    pub update_stage: u8,
    pub update_dialog: bool,
    pub update_error: Option<String>,
    pub update_checked: Option<String>,
    last_update_check: f64,
    last_poll: f64,
    last_refresh: f64,
    jobs_before: usize,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        let prefs = crate::prefs::load();
        set_theme(theme_index(&prefs.theme), prefs.accent);
        apply_style(&cc.egui_ctx);
        if prefs.scale != 1.0 {
            cc.egui_ctx.set_zoom_factor(prefs.scale);
        }
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let page = match std::env::var("TL_PAGE").as_deref() {
            Ok("library") => Page::Library,
            Ok("browse") => Page::Browse,
            Ok("skins") => Page::Skins,
            Ok("settings") => Page::Settings,
            _ => Page::Home,
        };
        let tab: usize = std::env::var("TL_TAB").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        Self {
            be: Backend::new(cc.egui_ctx.clone()),
            ready: false,
            init_error: None,
            page,
            back: vec![],
            fwd: vec![],
            page_changed: 0.0,
            instances: vec![],
            running: HashMap::new(),
            launching: HashMap::new(),
            jobs: vec![],
            content: HashMap::new(),
            logs: HashMap::new(),
            screens: HashMap::new(),
            accounts: vec![],
            default_user: None,
            login_busy: false,
            versions: vec![],
            lib_filter: 0,
            lib_search: String::new(),
            lib_sort: 0,
            browse_tab: 0,
            browse_query: String::new(),
            browse_sort: 0,
            browse_target: None,
            browse_match: true,
            browse_sig: String::new(),
            browse_edit_at: 0.0,
            browse_seq: 0,
            browse_offset: 0,
            browse_loading: false,
            hits: vec![],
            hits_total: 0,
            browse_error: None,
            installing: HashSet::new(),
            installed: HashSet::new(),
            inst_tab: tab,
            mod_search: String::new(),
            mod_filter: 0,
            skins: vec![],
            skin_tex: HashMap::new(),
            skin_equipped: HashMap::new(),
            skin_sel: None,
            skin_tab: 0,
            skin_yaw: 0.45,
            skin_walk: false,
            settings_section: tab,
            s: Settings { ram: 4.0 },
            prefs,
            cats: crate::cats::scan(),
            cat_seed: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            create: CreateDialog { loader: 1, ..Default::default() },
			import_open: std::env::var("TL_DIALOG").as_deref() == Ok("import"),
			dropped_mods: Vec::new(),
			drop_target: None,
            ely: ElyDialog::default(),
            nick: NickDialog::default(),
            rename: RenameDialog::default(),
            confirm_delete: None,
            toasts: vec![],
            last_poll: 0.0,
            last_refresh: 0.0,
            jobs_before: 0,
            perf: None,
            mod_info: None,
            mod_details: HashMap::new(),
            md_cache: HashMap::new(),
            inst_settings: None,
            export: None,
            update: None,
            project_view: None,
            proj_versions: HashMap::new(),
            categories: HashMap::new(),
            browse_cats: Default::default(),
            browse_source: 0,
            cf_key_edit: String::new(),
            java: None,
            launcher_dir: None,
            dir_pending: None,
            skins_checked_for: vec![],
            update_stage: 0,
            update_dialog: false,
            update_error: None,
            update_checked: None,
            last_update_check: -1e9,
        }
    }

    /// Применить тему/акцент из настроек и сохранить их.
    pub fn apply_look(&mut self, ctx: &egui::Context) {
        set_theme(theme_index(&self.prefs.theme), self.prefs.accent);
        apply_style(ctx);
        crate::prefs::save(&self.prefs);
    }

    /// Котик в правом нижнем углу страницы (под содержимым), как в Prism Launcher.
    fn draw_cat(&self, ui: &mut Ui) {
        if self.prefs.cat == crate::cats::NONE_ID || self.prefs.cat_opacity == 0 {
            return;
        }
        let Some(img) = self.cats.iter().find(|c| c.id == self.prefs.cat).and_then(|c| c.image(self.cat_seed)) else { return };
        let area = ui.max_rect();
        let low = img.uri.to_ascii_lowercase();
        let image = match img.bytes {
            Some(b) => egui::Image::from_bytes(img.uri, b),
            // GIF/WebP во время игры замирает на первом кадре — иначе окно перерисовывается с частотой анимации
            None if self.calm() && (low.ends_with(".gif") || low.ends_with(".webp")) => egui::Image::from_uri(format!("{}#0", img.uri)),
            None => egui::Image::from_uri(img.uri),
        };
        let Ok(egui::load::TexturePoll::Ready { texture }) = image.load_for_size(ui.ctx(), area.size()) else { return };
        let size = texture.size;
        if size.x < 1.0 || size.y < 1.0 {
            return;
        }
        let (aw, ah) = (area.width(), area.height());
        let mut uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let target = match self.prefs.cat_fit {
            // в углу: натуральный размер, но не больше 45 % ширины и 60 % высоты
            0 => {
                let k = (aw * 0.45 / size.x).min(ah * 0.6 / size.y).min(1.0);
                Rect::from_min_max(area.max - size * k, area.max)
            }
            // вписать целиком, прижав к правому нижнему углу
            1 => {
                let k = (aw / size.x).min(ah / size.y);
                Rect::from_min_max(area.max - size * k, area.max)
            }
            // заполнить с обрезкой
            2 => {
                let k = (aw / size.x).max(ah / size.y);
                let (vw, vh) = (aw / (size.x * k), ah / (size.y * k));
                uv = Rect::from_min_max(pos2(1.0 - vw, 1.0 - vh), pos2(1.0, 1.0));
                area
            }
            _ => area,
        };
        let tint = Color32::WHITE.gamma_multiply(self.prefs.cat_opacity as f32 / 100.0);
        image.uv(uv).tint(tint).paint_at(ui, target);
    }

    pub fn now(ui: &Ui) -> f64 {
        ui.input(|i| i.time)
    }

    pub fn inst(&self, id: &str) -> Option<&Inst> {
        self.instances.iter().find(|i| i.id == id)
    }

    pub fn go(&mut self, ui: &Ui, p: Page) {
        if p != self.page {
            self.back.push(self.page.clone());
            self.fwd.clear();
            self.on_enter(&p);
            self.page = p;
            self.page_changed = Self::now(ui);
        }
    }

    pub(crate) fn on_enter(&mut self, p: &Page) {
        if !self.ready {
            return;
        }
        match p {
            Page::Instance(id) => {
                self.inst_tab = 0;
                self.be.load_content(id.clone());
                self.browse_target = Some(id.clone());
            }
            Page::Skins => self.be.load_skins(),
            Page::Settings => self.be.load_users(),
            Page::Browse => {
                if let Some(t) = &self.browse_target {
                    self.be.installed_ids(t.clone());
                }
            }
            _ => {}
        }
    }

    pub fn toast(&mut self, ui: &Ui, text: impl Into<String>, _icon: Icon) {
        self.toasts.push(Toast { text: text.into(), error: false, born: Self::now(ui) });
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.running.contains_key(id)
    }

    pub fn is_launching(&self, id: &str, now: f64) -> bool {
        self.launching.get(id).is_some_and(|t| now - t < 25.0) && !self.is_running(id)
    }

    /// Играть / остановить.
    pub fn play(&mut self, ui: &Ui, id: &str) {
        let now = Self::now(ui);
        if self.is_running(id) {
            self.be.kill(id.to_string());
            self.launching.remove(id);
        } else if !self.is_launching(id, now) {
            if self.accounts.is_empty() {
                self.toast(ui, "Сначала войдите в аккаунт: Настройки → Аккаунты", Icon::User);
            }
            self.launching.insert(id.to_string(), now);
            self.be.run(id.to_string());
        }
    }

    pub fn default_account(&self) -> Option<&Account> {
        self.default_user.and_then(|d| self.accounts.iter().find(|a| a.id == d)).or(self.accounts.first())
    }

    fn handle_msgs(&mut self, ctx: &egui::Context, now: f64) {
        while let Ok(m) = self.be.rx.try_recv() {
            match m {
                Msg::Ready => {
                    self.ready = true;
                    self.be.refresh_instances();
                    self.be.load_users();
                    self.be.load_skins();
                    self.be.load_versions();
                    self.be.get_memory();
                    self.be.load_perf();
                    let p = self.page.clone();
                    self.on_enter(&p);
                }
                Msg::InitError(e) => self.init_error = Some(e),
                Msg::Instances(v) => {
                    if self.browse_target.is_none() {
                        self.browse_target = v.first().map(|i| i.id.clone());
                    }
                    if std::env::var("TL_PAGE").as_deref() == Ok("instance")
                        && self.page == Page::Home
                        && let Some(first) = v.first()
                    {
                        let tab = self.inst_tab;
                        self.page = Page::Instance(first.id.clone());
                        self.be.load_content(first.id.clone());
                        self.inst_tab = tab;
                    }
                    self.instances = v;
                    if let Page::Instance(id) = &self.page
                        && self.inst(id).is_none()
                    {
                        self.page = Page::Library;
                    }
                }
                Msg::Running(r) => {
                    for id in r.keys() {
                        self.launching.remove(id);
                    }
                    let stopped = self.running.keys().any(|k| !r.contains_key(k));
                    let (was, now_playing) = (!self.running.is_empty(), !r.is_empty());
                    self.running = r;
                    // поведение окна как в Prism: свернуть при запуске игры, вернуть после выхода
                    if let Some(p) = &self.perf {
                        if !was && now_playing && p.hide_on_start {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        if was && !now_playing && p.refocus_on_close {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        }
                    }
                    if stopped {
                        self.be.refresh_instances();
                    }
                }
                Msg::Jobs(j) => {
                    if j.len() < self.jobs_before {
                        self.be.refresh_instances();
                        if let Page::Instance(id) = &self.page {
                            self.be.load_content(id.clone());
                        }
                    }
                    self.jobs_before = j.len();
                    self.jobs = j;
                }
                Msg::Content(id, rows) => {
                    // для скриншотов: TL_DIALOG=modinfo|settings|export|ely открывает окно само
                    let first_load = !self.content.contains_key(&id);
                    self.content.insert(id.clone(), rows);
                    if first_load && let Ok(d) = std::env::var("TL_DIALOG") {
                        match d.as_str() {
                            "modinfo" => {
                                let pick = std::env::var("TL_MOD").unwrap_or_default().to_lowercase();
                                let row = self.content[&id].iter().find(|r| r.project_id.is_some() && r.name.to_lowercase().contains(&pick)).cloned();
                                if let Some(r) = row {
                                    self.open_mod_info(&id, &r);
                                }
                            }
                            "settings" => self.open_inst_settings(&id),
                            "export" => self.open_export(&id, crate::instcfg::ExportFormat::Prism),
                            "ely" => self.ely.open = true,
                            _ => {}
                        }
                    }
                }
                Msg::Search(seq, res) => {
                    if seq == self.browse_seq {
                        self.browse_loading = false;
                        match res {
                            Ok((hits, total)) => {
                                if self.browse_offset == 0 {
                                    if std::env::var("TL_DIALOG").as_deref() == Ok("project") && self.project_view.is_none()
                                        && let Some(h) = hits.get(2)
                                    {
                                        self.open_project(&h.clone());
                                    }
                                    self.hits = hits;
                                } else {
                                    self.hits.extend(hits);
                                }
                                self.hits_total = total;
                                self.browse_error = None;
                            }
                            Err(e) => self.browse_error = Some(e),
                        }
                    }
                }
                Msg::Users(list, def) => {
                    let new: Vec<Account> = list.iter().filter(|a| !self.skins_checked_for.contains(&a.id)).cloned().collect();
                    if !new.is_empty() {
                        self.skins_checked_for.extend(new.iter().map(|a| a.id));
                        self.be.ensure_account_skins(new);
                    }
                    self.accounts = list;
                    self.default_user = def;
                }
                Msg::ReloadSkins => self.be.load_skins(),
                Msg::Categories(k, v) => {
                    self.categories.insert(k, v);
                }
                Msg::Versions2(id, v) => {
                    self.proj_versions.insert(id, v);
                }
                Msg::Java(cur, found) => self.java = Some((cur, found)),
                Msg::JavaReload => self.be.load_java(),
                Msg::JavaPicked(major, path) => self.be.set_java(major, path),
                Msg::LauncherDir(d) => self.launcher_dir = d,
                Msg::LauncherDirPending(p) => self.dir_pending = Some(p),
                Msg::Skins(items, eq) => {
                    for it in &items {
                        if let Some((w, h, rgba)) = &it.rgba
                            && !self.skin_tex.contains_key(&it.id)
                            && *w == 64
                        {
                            let img = egui::ColorImage::from_rgba_unmultiplied([*w as usize, *h as usize], rgba);
                            let tex = ctx.load_texture(format!("skin-{}", it.id), img, egui::TextureOptions::NEAREST);
                            self.skin_tex.insert(it.id.clone(), SkinTex { tex, h: *h as f32 });
                        }
                    }
                    if self.skin_sel.as_ref().is_none_or(|s| !items.iter().any(|i| &i.id == s)) {
                        let acc = self.default_account().map(|a| a.id.to_string());
                        self.skin_sel = acc.and_then(|a| eq.get(&a).cloned()).or_else(|| items.first().map(|i| i.id.clone()));
                    }
                    self.skins = items;
                    self.skin_equipped = eq;
                }
                Msg::Log(id, text) => {
                    self.logs.insert(id, text);
                }
                Msg::Screens(id, v) => {
                    self.screens.insert(id, v);
                }
                Msg::Versions(v) => self.versions = v,
                Msg::Toast(text, error) => self.toasts.push(Toast { text, error, born: now }),
                Msg::InstallDone(pid) => {
                    if let Some(id) = pid.strip_prefix("have:") {
                        self.installed.insert(id.to_string());
                    } else {
                        if self.installing.remove(&pid) {
                            self.installed.insert(pid);
                        }
                        if let Page::Instance(id) = &self.page {
                            self.be.load_content(id.clone());
                        }
                    }
                }
                Msg::ElyDone => self.ely = ElyDialog::default(),
                Msg::Update(res, manual) => match res {
                    Ok(Some(info)) => {
                        if info.critical {
                            self.update_dialog = true;
                        }
                        self.update_checked = None;
                        self.update = Some(info);
                    }
                    Ok(None) => {
                        self.update_checked = Some("Стоит последняя версия".into());
                        if manual {
                            self.toasts.push(Toast { text: format!("Обновлений нет — у вас последняя версия {}", env!("CARGO_PKG_VERSION")), error: false, born: now });
                        }
                    }
                    Err(e) => {
                        self.update_checked = Some(format!("Не удалось проверить: {e}"));
                        if manual {
                            self.toasts.push(Toast { text: format!("Не удалось проверить обновления: {e}"), error: true, born: now });
                        }
                    }
                },
                Msg::UpdateInstalled(res) => match res {
                    Ok(()) => self.update_stage = 2,
                    Err(e) => {
                        self.update_stage = 0;
                        self.update_error = Some(e);
                    }
                },
                Msg::LoginBusy(b) => {
                    self.login_busy = b;
                    if !b {
                        self.be.load_users();
                        self.be.load_skins();
                        self.ely.open = false;
                    }
                }
                Msg::Picked(what, path) => match what {
                    Pick::InstanceIcon(id) => self.be.set_icon(id, path),
                    Pick::NewInstanceIcon => self.create.icon = Some(path),
                    Pick::SkinFile => self.be.add_skin_file(path),
                    Pick::CatImage => match crate::cats::add_own(&path) {
                        Ok(id) => {
                            self.cats = crate::cats::scan();
                            self.prefs.cat = id;
                            crate::prefs::save(&self.prefs);
                            self.toasts.push(Toast { text: "Котик добавлен".into(), error: false, born: now });
                        }
                        Err(e) => self.toasts.push(Toast { text: format!("Не удалось добавить котика: {e}"), error: true, born: now }),
                    },
                },
                Msg::Memory(mb) => self.s.ram = mb as f32 / 1024.0,
                Msg::Perf(p) => self.perf = Some(p),
                Msg::InstSettings(id, st) => {
                    if let Some(d) = self.inst_settings.as_mut()
                        && d.id == id
                    {
                        d.s = Some(st);
                    }
                }
                Msg::ExportRoot(id, e) => {
                    if let Some(d) = self.export.as_mut()
                        && d.id == id
                    {
                        // проверка экспорта без интерфейса: TL_EXPORT_TO + TL_EXPORT_FMT=prism|modrinth|curseforge
                        if let (Some(_), Ok(f)) = (std::env::var_os("TL_EXPORT_TO"), std::env::var("TL_EXPORT_FMT")) {
                            let fmt = match f.as_str() {
                                "modrinth" => crate::instcfg::ExportFormat::Modrinth,
                                "curseforge" => crate::instcfg::ExportFormat::CurseForge,
                                _ => crate::instcfg::ExportFormat::Prism,
                            };
                            let inc = e.iter().filter(|x| x.include).map(|x| x.name.clone()).collect();
                            d.busy = true;
                            self.be.export_instance(id.clone(), fmt, "1.0.0".into(), inc);
                        }
                        d.entries = Some(e);
                    }
                }
                Msg::ExportDone(res) => {
                    if let Some(d) = self.export.as_mut() {
                        d.busy = false;
                    }
                    match res {
                        Ok(Some(path)) => {
                            self.export = None;
                            self.toasts.push(Toast { text: format!("Сборка экспортирована: {}", path.display()), error: false, born: now });
                            if let Some(dir) = path.parent() {
                                crate::sys::open(dir);
                            }
                        }
                        Ok(None) => {}
                        Err(e) => self.toasts.push(Toast { text: format!("Экспорт не удался: {e}"), error: true, born: now }),
                    }
                }
                Msg::ModDetails(pid, d) => {
                    self.md_cache.retain(|k, _| !k.starts_with(&format!("{pid}#")));
                    self.mod_details.insert(pid, d);
                }
            }
        }
    }

    /// Тихий режим: идёт игра — никаких постоянных анимаций, окно почти не перерисовывается
    /// и не отнимает у игры CPU и видеокарту. Prism делает так же просто потому, что Qt не рисует без событий.
    pub fn calm(&self) -> bool {
        !self.running.is_empty() && self.perf.as_ref().is_none_or(|p| p.s.pause_ui_while_playing)
    }

    fn tick(&mut self, ui: &Ui) {
        let now = Self::now(ui);
        self.handle_msgs(&ui.ctx().clone(), now);
        if self.ready {
            if now - self.last_poll > 1.0 {
                self.last_poll = now;
                self.be.poll_status();
            }
            // во время игры список сборок трогаем реже — меньше обращений к базе и диску
            if now - self.last_update_check > 6.0 * 3600.0 && std::env::var_os("TL_NO_UPDATE").is_none() {
                self.last_update_check = now;
                self.be.check_update(false);
            }
            if now - self.last_refresh > if self.calm() { 30.0 } else { 6.0 } {
                self.last_refresh = now;
                self.be.refresh_instances();
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(if self.calm() { 1000 } else { 500 }));
        self.toasts.retain(|t| now - t.born < 4.5);
    }

    // ───────────────────────── картинки сборок ─────────────────────────

    /// Аватарка сборки: фото, если поставлено, иначе пиксельный узор.
    pub fn inst_icon(ui: &Ui, r: Rect, inst: &Inst, rounding: f32) {
        if let Some(p) = &inst.icon
            && std::path::Path::new(p).exists()
        {
            ui.painter().rect_filled(r, rounding, RAISED());
            egui::Image::from_uri(crate::sys::file_uri(std::path::Path::new(p))).corner_radius(rounding as u8).paint_at(ui, r);
        } else {
            let (seed, hue) = seed_of(&inst.id);
            pixel_icon(ui.painter(), r, seed, hue, rounding);
        }
    }

    // ───────────────────────── боковая панель ─────────────────────────

    fn sidebar(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let logo_r = Rect::from_center_size(pos2(full.center().x, full.top() + 26.0), vec2(24.0, 30.0));
        let lr = ui.interact(logo_r.expand(6.0), Id::new("logo"), Sense::click());
        logo(ui.painter(), logo_r);
        if lr.clicked() {
            self.go(ui, Page::Home);
        }
        ui.add_space(56.0);

        let items = [
            (Page::Home, Icon::Home, "Главная"),
            (Page::Library, Icon::Library, "Библиотека"),
            (Page::Browse, Icon::Browse, "Обзор Modrinth"),
            (Page::Skins, Icon::Skins, "Скины"),
        ];
        let mut active_y: Option<f32> = None;
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (p, ic, tip) in items {
                let active = self.page == p || (p == Page::Library && matches!(self.page, Page::Instance(_)));
                let (r, resp) = nav_button(ui, ic, active, tip);
                if active {
                    active_y = Some(r.center().y);
                }
                if resp.clicked() {
                    self.go(ui, p);
                }
            }
            ui.add_space(4.0);
            let y = ui.cursor().top();
            ui.painter().hline(full.center().x - 14.0..=full.center().x + 14.0, y, Stroke::new(1.0, BORDER()));
            ui.add_space(10.0);
			if let Some(y) = self.sidebar_instances(ui, full.bottom()) { active_y = Some(y); }
            let (_, resp) = nav_button(ui, Icon::Plus, false, "Создать сборку");
            if resp.clicked() {
                self.create.open = true;
            }
        });

        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            ui.add_space(12.0);
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
            let t = hover_anim(ui, resp.id, resp.hovered());
            let hr = r.shrink(6.0 - t);
            ui.painter().rect_filled(hr.expand(2.0), 7, HOVER().gamma_multiply(t));
            self.account_face(ui, hr);
            let tip = self.default_account().map(|a| format!("{} · {}", a.name, if a.elyby { "Ely.by" } else { "Microsoft" })).unwrap_or_else(|| "Войти в аккаунт".into());
            if resp.on_hover_text(tip).clicked() {
                self.settings_section = ACCOUNTS_SECTION;
                self.go(ui, Page::Settings);
            }
            ui.add_space(4.0);
            let active = self.page == Page::Settings;
            let (r, resp) = nav_button(ui, Icon::Settings, active, "Настройки");
            if active {
                active_y = Some(r.center().y);
            }
            if resp.clicked() {
                self.go(ui, Page::Settings);
            }
        });

        if let Some(y) = active_y {
            let y = ease_to(ui.ctx(), Id::new("nav-ind"), y, 0.24);
            let r = Rect::from_center_size(pos2(full.left() + 1.5, y), vec2(3.0, 20.0));
            ui.painter().rect_filled(r, CornerRadius { nw: 0, sw: 0, ne: 2, se: 2 }, accent());
        }
    }

    /// Надетый скин аккаунта: отметка в библиотеке, иначе скин с тем же именем.
    pub fn equipped_skin_of(&self, a: &Account) -> Option<String> {
        self.skin_equipped
            .get(&a.id.to_string())
            .cloned()
            .or_else(|| self.skins.iter().find(|s| s.name.eq_ignore_ascii_case(&a.name)).map(|s| s.id.clone()))
    }

    /// Лицо текущего аккаунта по надетому скину.
    pub fn account_face(&self, ui: &Ui, r: Rect) {
        let tex = self.default_account().and_then(|a| self.equipped_skin_of(a)).and_then(|id| self.skin_tex.get(&id));
        if let Some(t) = tex {
            crate::skin3d::draw_face(ui.painter(), r, t.tex.id(), t.h);
        } else if self.accounts.is_empty() {
            ui.painter().rect_filled(r, 6, RAISED());
            icon(ui.painter(), Icon::User, r.shrink(5.0), hex(0x7f8bfe));
        } else {
            head(ui.painter(), r, &DEFAULT_LOOK, 7);
        }
    }

    // ───────────────────────── верхняя строка ─────────────────────────

    fn topbar(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        let drag = ui.interact(r, Id::new("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
        }
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.add_space(6.0);
            ui.add_enabled_ui(!self.back.is_empty(), |ui| {
                if icon_button(ui, Icon::Back, 30.0, "Назад").clicked() {
					self.navigate_history(ui, false);
				}
            });
            ui.add_enabled_ui(!self.fwd.is_empty(), |ui| {
                if icon_button(ui, Icon::Forward, 30.0, "Вперёд").clicked() {
					self.navigate_history(ui, true);
				}
            });
            ui.add_space(10.0);
            let title = match &self.page {
                Page::Home => "Главная".to_string(),
                Page::Library => "Библиотека".into(),
                Page::Browse => "Обзор Modrinth".into(),
                Page::Skins => "Скины".into(),
                Page::Settings => "Настройки".into(),
                Page::Instance(id) => self.inst(id).map(|i| i.name.clone()).unwrap_or_default(),
            };
            if matches!(self.page, Page::Instance(_)) {
                let resp = ui.add(egui::Label::new(egui::RichText::new("Библиотека").font(medium(13.5)).color(TEXT_DIM())).sense(Sense::click()));
                if resp.clicked() {
                    self.go(ui, Page::Library);
                }
                label(ui, "  ›  ", regular(13.5), TEXT_FAINT());
            }
            label(ui, title, medium(13.5), TEXT());

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(6.0);
                if window_button(ui, Icon::Close, true).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                if window_button(ui, Icon::Max, false).clicked() {
                    let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
                }
                if window_button(ui, Icon::Min, false).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                ui.add_space(10.0);
                self.update_button(ui);
                self.downloads_button(ui);
                ui.add_space(8.0);
                self.running_pill(ui);
            });
        });
    }

    fn update_button(&mut self, ui: &mut Ui) {
        let Some(info) = &self.update else { return };
        let text = match self.update_stage {
            2 => "Перезапустить".to_string(),
            1 => "Обновляю…".to_string(),
            _ => format!("Обновить до {}", info.version),
        };
        let resp = button(ui, &text, Some(Icon::Download), Kind::Primary, 30.0).on_hover_text("Новая версия TrollLauncher");
        if resp.clicked() {
            if self.update_stage == 2 {
                crate::update::restart();
            }
            self.update_dialog = true;
        }
        ui.add_space(8.0);
    }

    pub fn update_dialog_ui(&mut self, ctx: &egui::Context) {
        if !self.update_dialog {
            return;
        }
        let Some(info) = self.update.clone() else {
            self.update_dialog = false;
            return;
        };
        let frame = Frame::new().fill(PANEL()).corner_radius(14).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(22));
        let r = egui::Modal::new(Id::new("update")).frame(frame).show(ctx, |ui| {
            ui.set_width(460.0);
            ui.spacing_mut().item_spacing.y = 10.0;
            label(ui, format!("TrollLauncher {}", info.version), bold(21.0), TEXT());
            label(ui, format!("Сейчас стоит {}. Сборки, аккаунты и настройки останутся на месте.", info.current), regular(13.5), TEXT_DIM());
            if !info.notes.is_empty() {
                egui::Frame::new().fill(INPUT()).corner_radius(10).inner_margin(12).show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let blocks = crate::modinfo::parse_md(&info.notes);
                        crate::modinfo::md_view(ui, &blocks);
                    });
                });
            }
            if let Some(e) = &self.update_error {
                ui.add(egui::Label::new(egui::RichText::new(format!("Не получилось: {e}")).font(regular(13.0)).color(lighten(DANGER(), 0.3))).wrap());
                ui.add(egui::Label::new(egui::RichText::new(crate::update::manual_hint()).font(mono(12.0)).color(TEXT_DIM())).wrap());
                if !cfg!(windows) && button(ui, "Скопировать команду", Some(Icon::Copy), Kind::Secondary, 30.0).clicked() {
                    ui.ctx().copy_text("curl -fsSL https://raw.githubusercontent.com/Flenstick/TrollLauncher/main/install.sh | bash".into());
                }
            }
            ui.horizontal(|ui| match self.update_stage {
                2 => {
                    if button(ui, "Перезапустить", Some(Icon::Refresh), Kind::Primary, 38.0).clicked() {
                        crate::update::restart();
                    }
                    if button(ui, "Позже", None, Kind::Secondary, 38.0).clicked() {
                        self.update_dialog = false;
                    }
                }
                1 => {
                    let now = Self::now(ui) as f32;
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                    let pts: Vec<_> = (0..=10).map(|i| r.center() + Vec2::angled(now * 5.0 + i as f32 * 0.2) * 8.0).collect();
                    ui.painter().add(egui::Shape::line(pts, Stroke::new(2.0, accent())));
                    ui.ctx().request_repaint();
                    let done = crate::update::DOWNLOADED.load(std::sync::atomic::Ordering::Relaxed) as f32 / 1_048_576.0;
                    let total = crate::update::TOTAL.load(std::sync::atomic::Ordering::Relaxed) as f32 / 1_048_576.0;
                    let text = if total > 0.0 && done < total {
                        format!("Скачиваю: {done:.0} из {total:.0} МБ")
                    } else if done > 0.0 {
                        "Ставлю…".to_string()
                    } else {
                        "Начинаю скачивание…".to_string()
                    };
                    label(ui, text, medium(14.0), TEXT());
                    if total > 0.0 {
                        let (r, _) = ui.allocate_exact_size(vec2(140.0, 6.0), Sense::hover());
                        progress_bar(ui.painter(), r, (done / total).clamp(0.0, 1.0));
                    }
                }
                _ => {
                    if button(ui, "Обновить", Some(Icon::Download), Kind::Primary, 38.0).clicked() {
                        self.update_error = None;
                        self.update_stage = 1;
                        self.be.install_update(info.clone());
                    }
                    if button(ui, "Позже", None, Kind::Secondary, 38.0).clicked() {
                        self.update_dialog = false;
                    }
                }
            });
        });
        if r.should_close() && self.update_stage != 1 {
            self.update_dialog = false;
        }
    }

    fn downloads_button(&mut self, ui: &mut Ui) {
        let n = self.jobs.len() + self.installing.len();
        let resp = icon_button(ui, Icon::Download, 32.0, "");
        if n > 0 {
            let c = resp.rect.right_top() + vec2(-7.0, 7.0);
            ui.painter().circle_filled(c, 7.0, accent());
            text_at(ui.painter(), c, Align2::CENTER_CENTER, n, semibold(10.0), on_accent());
        }
        let jobs: Vec<(String, Option<f32>)> = self
            .jobs
            .iter()
            .map(|j| (j.title.clone(), j.progress))
            .chain(self.installing.iter().map(|p| (self.hits.iter().find(|h| &h.project_id == p).map(|h| h.title.clone()).unwrap_or_else(|| p.clone()), None)))
            .collect();
        resp.on_hover_ui(|ui| {
            ui.set_min_width(240.0);
            if jobs.is_empty() {
                label(ui, "Загрузок нет", regular(13.0), TEXT_DIM());
            }
            for (t, p) in &jobs {
                label(ui, t, medium(13.0), TEXT());
                if let Some(p) = p {
                    let (r, _) = ui.allocate_exact_size(vec2(230.0, 4.0), Sense::hover());
                    progress_bar(ui.painter(), r, *p);
                }
            }
        });
    }

    fn running_pill(&mut self, ui: &mut Ui) {
        let now = Self::now(ui);
        let launching = self.launching.iter().find(|(id, t)| now - **t < 25.0 && !self.running.contains_key(*id)).map(|(id, _)| id.clone());
        let running = self.running.iter().next().map(|(id, t)| (id.clone(), *t));
        let (id, label_s, is_launch) = if let Some((id, start)) = running {
            let s = (Utc::now() - start).num_seconds().max(0) as u64;
            let name = self.inst(&id).map(|i| i.name.clone()).unwrap_or_default();
            (id, format!("{name} · {:02}:{:02}", s / 60, s % 60), false)
        } else if let Some(id) = launching {
            let name = self.inst(&id).map(|i| i.name.clone()).unwrap_or_default();
            (id, format!("{name} · запуск…"), true)
        } else {
            return;
        };
        let g = ui.painter().layout_no_wrap(label_s, medium(13.0), TEXT());
        let w = g.size().x + 26.0 + 36.0;
        let (r, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
        let p = ui.painter();
        p.rect(r, 9, PANEL(), Stroke::new(1.0, BORDER()), StrokeKind::Inside);
        let dot = pos2(r.left() + 14.0, r.center().y);
        if is_launch {
            let a = now as f32 * 6.0;
            p.circle_stroke(dot, 5.0, Stroke::new(1.5, HOVER()));
            let pts: Vec<_> = (0..=10).map(|i| dot + Vec2::angled(a + i as f32 * 0.18) * 5.0).collect();
            p.add(egui::Shape::line(pts, Stroke::new(1.8, accent())));
            ui.ctx().request_repaint();
        } else if self.calm() {
            // тихий режим: без пульса, перерисовка только раз в секунду для таймера
            p.circle_filled(dot, 4.0, GREEN());
        } else {
            let pulse = ((now * 2.5).sin() * 0.5 + 0.5) as f32;
            p.circle_filled(dot, 4.0 + pulse * 3.0, GREEN().gamma_multiply(0.25 * (1.0 - pulse)));
            p.circle_filled(dot, 4.0, GREEN());
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
        p.galley(pos2(r.left() + 26.0, r.center().y - g.size().y / 2.0), g, TEXT());
        let sr = Rect::from_center_size(pos2(r.right() - 18.0, r.center().y), Vec2::splat(24.0));
        let stop = ui.interact(sr, Id::new("pill-stop"), Sense::click());
        let t = hover_anim(ui, stop.id, stop.hovered());
        ui.painter().rect_filled(sr, 6, DANGER().gamma_multiply(0.15 + t * 0.85));
        icon(ui.painter(), Icon::Stop, sr.shrink(6.0), lighten(DANGER(), 0.5).lerp_to_gamma(Color32::WHITE, t));
        if stop.on_hover_text("Остановить").clicked() {
            self.be.kill(id.clone());
            self.launching.remove(&id);
        } else if resp.clicked() {
            self.go(ui, Page::Instance(id));
        }
    }

    fn toasts_ui(&mut self, ui: &mut Ui) {
        let now = Self::now(ui);
        if self.toasts.is_empty() {
            return;
        }
        ui.ctx().request_repaint();
        egui::Area::new(Id::new("toasts"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-18.0, -18.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ui.ctx(), |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for t in self.toasts.iter().rev() {
                    let age = (now - t.born) as f32;
                    let a_in = smooth((age / 0.22).min(1.0));
                    let a_out = smooth(((4.5 - age) / 0.3).clamp(0.0, 1.0));
                    let a = a_in * a_out;
                    let mut job = egui::text::LayoutJob::simple(t.text.clone(), medium(13.5), TEXT(), 420.0);
                    job.wrap.max_rows = 3;
                    let g = ui.painter().layout_job(job);
                    let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 58.0, (g.size().y + 24.0).max(44.0)), Sense::hover());
                    let r = r.translate(vec2((1.0 - a_in) * 30.0, 0.0));
                    let p = ui.painter();
                    let col = if t.error { DANGER() } else { accent() };
                    p.rect(r, 10, RAISED().gamma_multiply(a), Stroke::new(1.0, HOVER().gamma_multiply(a)), StrokeKind::Inside);
                    let ir = Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), Vec2::splat(26.0));
                    p.rect_filled(ir, 7, col.gamma_multiply(0.18 * a));
                    icon(p, if t.error { Icon::Close } else { Icon::Check }, ir.shrink(5.0), col.gamma_multiply(a));
                    p.galley(pos2(r.left() + 44.0, r.center().y - g.size().y / 2.0), g, TEXT().gamma_multiply(a));
                }
            });
    }

    // ───────────────────────── диалоги ─────────────────────────

    fn dialogs(&mut self, ctx: &egui::Context) {
        self.mod_info_dialog(ctx);
        self.project_dialog(ctx);
        self.inst_dialogs(ctx);
		self.import_dialog(ctx);
        self.update_dialog_ui(ctx);
        let modal_frame = Frame::new().fill(PANEL()).corner_radius(14).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(22));

        if self.create.open {
            let r = egui::Modal::new(Id::new("create")).frame(modal_frame).show(ctx, |ui| {
                ui.set_width(440.0);
                ui.spacing_mut().item_spacing.y = 10.0;
                label(ui, "Новая сборка", bold(20.0), TEXT());
                ui.horizontal(|ui| {
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(72.0), Sense::click());
                    let t = hover_anim(ui, resp.id, resp.hovered());
                    if let Some(p) = &self.create.icon {
                        egui::Image::from_uri(crate::sys::file_uri(p)).corner_radius(14).paint_at(ui, r);
                    } else {
                        ui.painter().rect_filled(r, 14, RAISED().lerp_to_gamma(HOVER(), t));
                        icon(ui.painter(), Icon::Image, Rect::from_center_size(r.center(), Vec2::splat(26.0)), TEXT_DIM());
                    }
                    if resp.on_hover_text("Выбрать картинку").clicked() {
                        self.be.pick(Pick::NewInstanceIcon);
                    }
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        label(ui, "Название", semibold(13.0), TEXT_DIM());
                        ui.add(egui::TextEdit::singleline(&mut self.create.name).hint_text("Моя сборка").desired_width(330.0).font(regular(14.0)).margin(Margin::symmetric(10, 8)));
                    });
                });
                label(ui, "Загрузчик", semibold(13.0), TEXT_DIM());
                segmented(ui, Id::new("cr-loader"), &["Vanilla", "Fabric", "Forge", "NeoForge", "Quilt"], &mut self.create.loader);
                label(ui, "Версия игры", semibold(13.0), TEXT_DIM());
                let cur = self.versions.get(self.create.version).cloned().unwrap_or_else(|| "загрузка…".into());
                egui::ComboBox::from_id_salt("cr-ver").selected_text(cur).width(200.0).height(320.0).show_ui(ui, |ui| {
                    for (i, v) in self.versions.iter().enumerate() {
                        ui.selectable_value(&mut self.create.version, i, v);
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let ok = !self.create.name.trim().is_empty() && !self.versions.is_empty();
                    ui.add_enabled_ui(ok, |ui| {
                        if button(ui, "Создать", Some(Icon::Plus), Kind::Primary, 36.0).clicked() {
                            let v = self.versions[self.create.version].clone();
                            self.be.create_instance(self.create.name.trim().to_string(), v, self.create.loader, self.create.icon.take());
                            self.create.open = false;
                            self.create.name.clear();
                        }
                    });
                    if button(ui, "Отмена", None, Kind::Secondary, 36.0).clicked() {
                        self.create.open = false;
                    }
                });
            });
            if r.should_close() {
                self.create.open = false;
            }
        }

        if self.ely.open {
            let oauth = Backend::elyby_oauth_ready();
            let r = egui::Modal::new(Id::new("ely")).frame(modal_frame).show(ctx, |ui| {
                ui.set_width(400.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                label(ui, "Вход через Ely.by", bold(20.0), TEXT());
                if oauth {
                    label(ui, "Откроется страница Ely.by в браузере — войдите там и нажмите «Разрешить». Пароль лаунчер не видит.", regular(13.5), TEXT_DIM());
                    ui.add_space(4.0);
                    if self.login_busy {
                        ui.horizontal(|ui| {
                            let now = Self::now(ui) as f32;
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                            let pts: Vec<_> = (0..=10).map(|i| r.center() + Vec2::angled(now * 5.0 + i as f32 * 0.2) * 7.0).collect();
                            ui.painter().add(egui::Shape::line(pts, Stroke::new(2.0, accent())));
                            ui.ctx().request_repaint();
                            label(ui, "Жду подтверждения в браузере…", medium(14.0), TEXT());
                        });
                        ui.horizontal(|ui| {
                            if button(ui, "Открыть ещё раз", Some(Icon::Globe), Kind::Secondary, 34.0).clicked() {
                                self.be.elyby_cancel();
                                self.be.elyby_browser_login();
                            }
                            if button(ui, "Отмена", None, Kind::Secondary, 34.0).clicked() {
                                self.be.elyby_cancel();
                            }
                        });
                    } else if button(ui, "  Войти через сайт Ely.by  ", Some(Icon::Globe), Kind::Primary, 42.0).clicked() {
                        self.be.elyby_browser_login();
                    }
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(egui::RichText::new("Войти по паролю").font(medium(13.0)).color(TEXT_DIM())).id_salt("ely-pass").show(ui, |ui| self.ely_password_form(ui));
                } else {
                    self.ely_password_form(ui);
                    label(ui, "Вход через браузер включится, когда в лаунчер будет вписан Client ID приложения Ely.by (см. README).", regular(12.0), TEXT_FAINT());
                }
                ui.add_space(4.0);
                if !self.login_busy && button(ui, "Закрыть", None, Kind::Secondary, 34.0).clicked() {
                    self.ely.open = false;
                }
            });
            if r.should_close() {
                if self.login_busy {
                    self.be.elyby_cancel();
                }
                self.ely.open = false;
            }
        }

        if self.nick.open {
            let r = egui::Modal::new(Id::new("nick")).frame(modal_frame).show(ctx, |ui| {
                ui.set_width(360.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                label(ui, "Скин по нику", bold(20.0), TEXT());
                segmented(ui, Id::new("nick-src"), &["Minecraft", "Ely.by"], &mut self.nick.source);
                let resp = ui.add(egui::TextEdit::singleline(&mut self.nick.nick).hint_text("Ник игрока").desired_width(f32::INFINITY).margin(Margin::symmetric(10, 8)));
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if (button(ui, "Добавить", Some(Icon::Plus), Kind::Primary, 36.0).clicked() || enter) && !self.nick.nick.trim().is_empty() {
                        self.be.add_skin_nick(self.nick.nick.trim().to_string(), self.nick.source == 1);
                        self.nick.open = false;
                        self.nick.nick.clear();
                    }
                    if button(ui, "Отмена", None, Kind::Secondary, 36.0).clicked() {
                        self.nick.open = false;
                    }
                });
            });
            if r.should_close() {
                self.nick.open = false;
            }
        }

        if self.rename.open {
            let r = egui::Modal::new(Id::new("rename")).frame(modal_frame).show(ctx, |ui| {
                ui.set_width(360.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                label(ui, "Переименовать сборку", bold(20.0), TEXT());
                let resp = ui.add(egui::TextEdit::singleline(&mut self.rename.name).desired_width(f32::INFINITY).margin(Margin::symmetric(10, 8)));
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if (button(ui, "Сохранить", None, Kind::Primary, 36.0).clicked() || enter) && !self.rename.name.trim().is_empty() {
                        self.be.rename(self.rename.id.clone(), self.rename.name.trim().to_string());
                        self.rename.open = false;
                    }
                    if button(ui, "Отмена", None, Kind::Secondary, 36.0).clicked() {
                        self.rename.open = false;
                    }
                });
            });
            if r.should_close() {
                self.rename.open = false;
            }
        }

        if let Some(id) = self.confirm_delete.clone() {
            let name = self.inst(&id).map(|i| i.name.clone()).unwrap_or_default();
            let r = egui::Modal::new(Id::new("del")).frame(modal_frame).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                label(ui, format!("Удалить «{name}»?"), bold(19.0), TEXT());
                label(ui, "Папка сборки с мирами и модами будет удалена с диска.", regular(13.5), TEXT_DIM());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if button(ui, "Удалить", None, Kind::Danger, 36.0).clicked() {
                        self.be.remove_instance(id.clone());
                        self.confirm_delete = None;
                        self.page = Page::Library;
                    }
                    if button(ui, "Отмена", None, Kind::Secondary, 36.0).clicked() {
                        self.confirm_delete = None;
                    }
                });
            });
            if r.should_close() {
                self.confirm_delete = None;
            }
        }
    }

    fn ely_password_form(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing.y = 8.0;
        label(ui, "Ник или e-mail", semibold(13.0), TEXT_DIM());
        ui.add(egui::TextEdit::singleline(&mut self.ely.user).desired_width(f32::INFINITY).margin(Margin::symmetric(10, 8)));
        label(ui, "Пароль", semibold(13.0), TEXT_DIM());
        ui.horizontal(|ui| {
            let w = ui.available_width() - 40.0;
            ui.add(egui::TextEdit::singleline(&mut self.ely.pass).password(!self.ely.show_pass).desired_width(w).margin(Margin::symmetric(10, 8)));
            let tip = if self.ely.show_pass { "Скрыть пароль" } else { "Показать пароль" };
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
            let t = hover_anim(ui, resp.id, resp.hovered());
            ui.painter().rect_filled(r, 8, RAISED().lerp_to_gamma(HOVER(), t));
            eye_icon(ui.painter(), r.shrink(8.0), if self.ely.show_pass { accent() } else { TEXT_DIM() }, !self.ely.show_pass);
            if resp.on_hover_text(tip).clicked() {
                self.ely.show_pass = !self.ely.show_pass;
            }
        });
        label(ui, "Код 2FA (если включён)", semibold(13.0), TEXT_DIM());
        ui.add(egui::TextEdit::singleline(&mut self.ely.totp).desired_width(f32::INFINITY).margin(Margin::symmetric(10, 8)));
        ui.add_enabled_ui(!self.login_busy && !self.ely.user.is_empty() && !self.ely.pass.is_empty(), |ui| {
            if button(ui, if self.login_busy { "Вход…" } else { "Войти" }, None, Kind::Primary, 36.0).clicked() {
                let totp = Some(self.ely.totp.trim().to_string()).filter(|s| !s.is_empty());
                self.be.elyby_login(self.ely.user.trim().to_string(), self.ely.pass.clone(), totp);
                self.ely.pass.clear();
            }
        });
    }

    fn splash(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        let c = r.center();
        logo(ui.painter(), Rect::from_center_size(c - vec2(0.0, 40.0), vec2(48.0, 62.0)));
        if let Some(e) = &self.init_error {
            text_at(ui.painter(), c + vec2(0.0, 20.0), Align2::CENTER_CENTER, "Не удалось запустить движок лаунчера", semibold(16.0), TEXT());
            text_wrap(ui.painter(), pos2(c.x - 260.0, c.y + 40.0), e, regular(13.0), TEXT_DIM(), 520.0, 6);
        } else {
            let now = Self::now(ui) as f32;
            let pts: Vec<_> = (0..=14).map(|i| c + vec2(0.0, 30.0) + Vec2::angled(now * 5.0 + i as f32 * 0.2) * 9.0).collect();
            ui.painter().add(egui::Shape::line(pts, Stroke::new(2.0, accent())));
            ui.ctx().request_repaint();
        }
    }

}

pub const DEFAULT_LOOK: SkinLook = SkinLook { skin: 0x7f8bfe, hair: 0x2f2cf8, shirt: 0x2f2cf8, pants: 0x1e1e3a, shoes: 0x15152a, eyes: 0x1c1c34 };

pub fn seed_of(id: &str) -> (u64, f32) {
    let mut h: u64 = 1469598103934665603;
    for b in id.bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    (h, (h % 1000) as f32 / 1000.0)
}

pub fn fmt_played(secs: u64) -> String {
    let h = secs / 3600;
    if h > 0 { format!("{h} ч") } else { format!("{} мин", secs / 60) }
}

pub fn fmt_ago(t: Option<DateTime<Utc>>) -> String {
    let Some(t) = t else { return "не запускалась".into() };
    let d = Utc::now() - t;
    let m = d.num_minutes();
    if m < 1 {
        "только что".into()
    } else if m < 60 {
        format!("{m} мин назад")
    } else if m < 60 * 24 {
        format!("{} ч назад", m / 60)
    } else if m < 60 * 24 * 2 {
        "вчера".into()
    } else if m < 60 * 24 * 30 {
        format!("{} дн. назад", m / 60 / 24)
    } else {
        t.format("%d.%m.%Y").to_string()
    }
}

fn nav_button(ui: &mut Ui, ic: Icon, active: bool, tip: &str) -> (Rect, egui::Response) {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(42.0), Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    let a = ui.ctx().animate_bool_with_time(resp.id.with("act"), active, 0.18);
    let bg = Color32::TRANSPARENT.lerp_to_gamma(HOVER(), (t * 0.8).max(a));
    ui.painter().rect_filled(r, 10, bg);
    let col = TEXT_DIM().lerp_to_gamma(TEXT(), t.max(a));
    icon(ui.painter(), ic, Rect::from_center_size(r.center(), Vec2::splat(21.0)), if active { accent().lerp_to_gamma(TEXT(), 0.15) } else { col });
    let resp = resp.on_hover_text(tip);
    (r, resp)
}

fn window_button(ui: &mut Ui, ic: Icon, close: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(34.0, 30.0), Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    let bg = if close { DANGER() } else { HOVER() };
    ui.painter().rect_filled(r, 7, Color32::TRANSPARENT.lerp_to_gamma(bg, t));
    icon(ui.painter(), ic, Rect::from_center_size(r.center(), Vec2::splat(15.0)), TEXT_DIM().lerp_to_gamma(if close { Color32::WHITE } else { TEXT() }, t));
    resp
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.tick(ui);
		ui.painter().rect_filled(ui.max_rect(), 0, BG());
		self.mouse_history(ui);
		self.file_drop(ui.ctx());

        egui::Panel::left("nav")
            .exact_size(64.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(BG()))
            .show_inside(ui, |ui| self.sidebar(ui));

        egui::Panel::top("top")
            .exact_size(52.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(BG()))
            .show_inside(ui, |ui| self.topbar(ui));

        egui::CentralPanel::default()
            .frame(Frame::new().fill(CONTENT()).corner_radius(CornerRadius { nw: 14, ne: 0, sw: 0, se: 0 }).stroke(Stroke::new(1.0, BORDER())))
            .show_inside(ui, |ui| {
                if !self.ready {
                    self.splash(ui);
                    return;
                }
				if self.page != Page::Home { self.draw_cat(ui); }
                let now = Self::now(ui);
                let t = if self.prefs.animations { smooth((((now - self.page_changed) / 0.22) as f32).clamp(0.0, 1.0)) } else { 1.0 };
                if t < 1.0 {
                    ui.ctx().request_repaint();
                }
                let page = self.page.clone();
                egui::ScrollArea::vertical().id_salt(format!("{page:?}")).auto_shrink(false).show(ui, |ui| {
                    ui.multiply_opacity(t);
                    Frame::new()
                        .inner_margin(Margin { left: 32, right: 32, top: (26.0 + (1.0 - t) * 10.0) as i8, bottom: 40 })
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            match page {
                                Page::Home => self.page_home(ui),
                                Page::Library => self.page_library(ui),
                                Page::Browse => self.page_browse(ui),
                                Page::Skins => self.page_skins(ui),
                                Page::Settings => self.page_settings(ui),
                                Page::Instance(id) => self.page_instance(ui, &id),
                            }
                        });
                });
            });

        let ctx = ui.ctx().clone();
        self.dialogs(&ctx);
        self.toasts_ui(ui);
		self.file_drop_dialog(&ctx);
    }
}
