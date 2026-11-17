//! Меню «⋯» сборки: окно настроек сборки (память, Java, окно игры, хуки) и экспорт
//! в Prism Launcher (zip), Modrinth (mrpack) и CurseForge (zip).

use crate::app::*;
use crate::backend::{Backend, Msg};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Id, Margin, Stroke, Ui};
use std::io::Write;
use std::path::{Path, PathBuf};
use theseus::prelude::*;

// ───────────────────────── данные ─────────────────────────

/// Настройки одной сборки; `None` — берётся общая настройка лаунчера.
#[derive(Clone, Debug, Default)]
pub struct InstSettings {
    pub name: String,
    pub memory_mb: Option<u32>,
    pub java_args: Option<String>,
    pub env_vars: Option<String>,
    pub java_path: Option<String>,
    pub fullscreen: Option<bool>,
    pub resolution: Option<(u16, u16)>,
    pub pre_launch: String,
    pub wrapper: String,
    pub post_exit: String,
    // общие значения — подсказки, что будет без переопределения
    pub global_memory_mb: u32,
    pub global_args: String,
    pub global_res: (u16, u16),
    pub game_version: String,
    pub loader: String,
    pub loader_version: String,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ExportFormat {
    Prism,
    Modrinth,
    CurseForge,
}

impl ExportFormat {
    pub fn title(self) -> &'static str {
        match self {
            Self::Prism => "Prism Launcher (zip)",
            Self::Modrinth => "Modrinth (mrpack)",
            Self::CurseForge => "CurseForge (zip)",
        }
    }
    fn ext(self) -> &'static str {
        if self == Self::Modrinth { "mrpack" } else { "zip" }
    }
}

#[derive(Clone, Debug)]
pub struct RootEntry {
    pub name: String,
    pub dir: bool,
    pub size: u64,
    pub include: bool,
}

pub struct ExportDialog {
    pub id: String,
    pub format: ExportFormat,
    pub version: String,
    pub entries: Option<Vec<RootEntry>>,
    pub busy: bool,
}

pub struct SettingsDialog {
    pub id: String,
    pub s: Option<InstSettings>,
    pub section: usize,
}

/// Что брать в архив по умолчанию: всё, что делает сборку сборкой, без миров, логов и кэшей.
const DEFAULT_ON: &[&str] = &[
    "mods", "resourcepacks", "shaderpacks", "config", "defaultconfigs", "kubejs", "scripts", "global_packs", "datapacks", "options.txt",
    "optionsof.txt", "optionsshaders.txt", "servers.dat", "emotes", "patchouli_books",
];
const NEVER: &[&str] = &["logs", "crash-reports", ".cache", ".fabric", ".mixin.out", "natives", "usercache.json", "usernamecache.json", "launcher_log.txt"];

fn dir_size(p: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![p.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(e.path()),
                Ok(_) => total += e.metadata().map(|m| m.len()).unwrap_or(0),
                _ => {}
            }
        }
    }
    total
}

fn human(b: u64) -> String {
    match b {
        0..=1023 => format!("{b} Б"),
        1024..=1_048_575 => format!("{:.0} КБ", b as f64 / 1024.0),
        1_048_576..=1_073_741_823 => format!("{:.1} МБ", b as f64 / 1_048_576.0),
        _ => format!("{:.2} ГБ", b as f64 / 1_073_741_824.0),
    }
}

fn loader_meta(loader: ModLoader, mc: &str, ver: &str) -> (Vec<(&'static str, String)>, Option<String>) {
    // Prism: компоненты mmc-pack.json; CurseForge: id загрузчика в manifest.json
    let v = ver.strip_prefix(&format!("{mc}-")).unwrap_or(ver).to_string();
    match loader {
        ModLoader::Vanilla => (vec![], None),
        ModLoader::Fabric => (vec![("net.fabricmc.intermediary", mc.to_string()), ("net.fabricmc.fabric-loader", v.clone())], Some(format!("fabric-{v}"))),
        ModLoader::Quilt => (vec![("net.fabricmc.intermediary", mc.to_string()), ("org.quiltmc.quilt-loader", v.clone())], Some(format!("quilt-{v}"))),
        ModLoader::Forge => (vec![("net.minecraftforge", v.clone())], Some(format!("forge-{v}"))),
        ModLoader::NeoForge => (vec![("net.neoforged", v.clone())], Some(format!("neoforge-{v}"))),
    }
}

/// Кладёт файл или папку из сборки в архив под префиксом.
fn zip_add<W: Write + std::io::Seek>(zw: &mut zip::ZipWriter<W>, base: &Path, rel: &str, prefix: &str) -> std::io::Result<()> {
    let deflate = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
    // уже сжатое (jar, картинки, звуки) кладём как есть — в разы быстрее, размер почти тот же
    let store = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored).large_file(true);
    let mut stack = vec![rel.to_string()];
    while let Some(r) = stack.pop() {
        let p = base.join(&r);
        let meta = std::fs::metadata(&p)?;
        if meta.is_dir() {
            for e in std::fs::read_dir(&p)?.flatten() {
                stack.push(format!("{r}/{}", e.file_name().to_string_lossy()));
            }
        } else {
            let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            let packed = matches!(ext.as_str(), "jar" | "zip" | "png" | "jpg" | "jpeg" | "ogg" | "mp3" | "gz" | "xz" | "zst" | "mrpack" | "disabled" | "webp");
            zw.start_file(format!("{prefix}{r}"), if packed { store } else { deflate }).map_err(std::io::Error::other)?;
            let mut f = std::fs::File::open(&p)?;
            std::io::copy(&mut f, zw)?;
        }
    }
    Ok(())
}

// ───────────────────────── движок ─────────────────────────

impl Backend {
    pub fn load_inst_settings(&self, id: String) {
        self.task(|tx| async move {
            let (Ok(Some(m)), Ok(g)) = (instance::get(&id).await, settings::get().await) else {
                let _ = tx.send(Msg::Toast("Не удалось прочитать настройки сборки".into(), true));
                return;
            };
            let o = m.launch_overrides;
            let cs = m.applied_content_set;
            let s = InstSettings {
                name: m.instance.name,
                memory_mb: o.memory.map(|x| x.maximum),
                java_args: o.extra_launch_args.map(|a| a.join(" ")),
                env_vars: o.custom_env_vars.map(|v| v.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("\n")),
                java_path: o.java_path,
                fullscreen: o.force_fullscreen,
                resolution: o.game_resolution.map(|w| (w.0, w.1)),
                pre_launch: o.hooks.pre_launch.unwrap_or_default(),
                wrapper: o.hooks.wrapper.unwrap_or_default(),
                post_exit: o.hooks.post_exit.unwrap_or_default(),
                global_memory_mb: g.memory.maximum,
                global_args: g.extra_launch_args.join(" "),
                global_res: (g.game_resolution.0, g.game_resolution.1),
                game_version: cs.game_version,
                loader: crate::backend::loader_name(cs.loader),
                loader_version: cs.loader_version.unwrap_or_default(),
            };
            let _ = tx.send(Msg::InstSettings(id, s));
        });
    }

    pub fn save_inst_settings(&self, id: String, s: InstSettings) {
        self.task(|tx| async move {
            let split = |t: &str| t.split_whitespace().map(String::from).collect::<Vec<_>>();
            let env = |t: &str| {
                t.lines()
                    .filter_map(|l| l.split_once('='))
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                    .filter(|(k, _)| !k.is_empty())
                    .collect::<Vec<_>>()
            };
            let none_if_empty = |t: &str| Some(t.trim().to_string()).filter(|x| !x.is_empty());
            let patch = EditInstance {
                name: Some(s.name.trim().to_string()).filter(|n| !n.is_empty()),
                launch_overrides: Some(InstanceLaunchOverridesPatch {
                    java_path: Some(s.java_path.as_deref().and_then(none_if_empty)),
                    extra_launch_args: Some(s.java_args.as_deref().map(split)),
                    custom_env_vars: Some(s.env_vars.as_deref().map(env)),
                    memory: Some(s.memory_mb.map(|maximum| MemorySettings { maximum })),
                    force_fullscreen: Some(s.fullscreen),
                    game_resolution: Some(s.resolution.map(|(w, h)| WindowSize(w, h))),
                    hooks: Some(Hooks { pre_launch: none_if_empty(&s.pre_launch), wrapper: none_if_empty(&s.wrapper), post_exit: none_if_empty(&s.post_exit) }),
                    visible_tabs: None,
                }),
                ..Default::default()
            };
            match instance::edit(&id, patch).await {
                Ok(_) => {
                    let _ = tx.send(Msg::Toast("Настройки сборки сохранены".into(), false));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Toast(format!("Настройки сборки: {e}"), true));
                }
            }
        });
        self.refresh_instances();
    }

    pub fn list_export_root(&self, id: String) {
        self.task(|tx| async move {
            let Ok(dir) = instance::get_full_path(&id).await else { return };
            let entries = tokio::task::spawn_blocking(move || {
                let mut v: Vec<RootEntry> = std::fs::read_dir(&dir)
                    .map(|rd| {
                        rd.flatten()
                            .filter_map(|e| {
                                let name = e.file_name().to_string_lossy().to_string();
                                if NEVER.contains(&name.as_str()) || name.starts_with('.') {
                                    return None;
                                }
                                let dir = e.file_type().ok()?.is_dir();
                                let size = if dir { dir_size(&e.path()) } else { e.metadata().map(|m| m.len()).unwrap_or(0) };
                                Some(RootEntry { include: DEFAULT_ON.contains(&name.as_str()), name, dir, size })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                v.sort_by(|a, b| b.dir.cmp(&a.dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
                v
            })
            .await
            .unwrap_or_default();
            let _ = tx.send(Msg::ExportRoot(id, entries));
        });
    }

    pub fn export_instance(&self, id: String, format: ExportFormat, version: String, include: Vec<String>) {
        self.task(|tx| async move {
            let res = async {
                let m = instance::get(&id).await.map_err(|e| e.to_string())?.ok_or("Сборка не найдена")?;
                let name = m.instance.name.clone();
                let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' }).collect();
                let file_name = format!("{} {version}.{}", safe.trim(), format.ext());
                // TL_EXPORT_TO — папка для проверки без диалога
                let picked = match std::env::var_os("TL_EXPORT_TO") {
                    Some(dir) => Some(PathBuf::from(dir).join(&file_name)),
                    None => rfd::AsyncFileDialog::new()
                        .set_title(format!("Экспорт — {}", format.title()))
                        .set_file_name(&file_name)
                        .add_filter(format.title(), &[format.ext()])
                        .save_file()
                        .await
                        .map(|f| f.path().to_path_buf()),
                };
                let Some(mut out) = picked else {
                    return Ok(None);
                };
                if out.extension().is_none_or(|e| e != format.ext()) {
                    out.set_extension(format.ext());
                }
                let base = instance::get_full_path(&id).await.map_err(|e| e.to_string())?;
                let cs = m.applied_content_set;
                let mc = cs.game_version.clone();
                let (components, cf_loader) = loader_meta(cs.loader, &mc, cs.loader_version.as_deref().unwrap_or_default());
                match format {
                    ExportFormat::Modrinth => {
                        instance::export_mrpack(&id, out.clone(), include, vec![], Some(version), None, Some(name)).await.map_err(|e| e.to_string())?;
                    }
                    ExportFormat::Prism | ExportFormat::CurseForge => {
                        let o = m.launch_overrides;
						let icon_path = m.instance.icon_path.clone().map(PathBuf::from);
                        let out2 = out.clone();
                        let tmp = PathBuf::from(format!("{}.part", out2.display()));
                        let tmp2 = tmp.clone();
                        let done = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
                            let tmp = tmp2;
                            let mut zw = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
                            let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
                            let prefix = if format == ExportFormat::Prism {
                                let mut comps = vec![serde_json::json!({"uid": "net.minecraft", "version": mc, "important": true})];
                                comps.extend(components.iter().map(|(uid, v)| serde_json::json!({"uid": uid, "version": v})));
                                zw.start_file("mmc-pack.json", opts)?;
                                zw.write_all(serde_json::to_string_pretty(&serde_json::json!({"components": comps, "formatVersion": 1}))?.as_bytes())?;
                                let mut cfg = format!("[General]\nConfigVersion=1.2\nInstanceType=OneSix\niconKey=icon\nname={name}\n");
                                if let Some(mem) = &o.memory {
                                    cfg += &format!("OverrideMemory=true\nMaxMemAlloc={}\nMinMemAlloc=512\n", mem.maximum);
                                }
                                if let Some(a) = &o.extra_launch_args {
                                    cfg += &format!("OverrideJavaArgs=true\nJvmArgs={}\n", a.join(" "));
                                }
                                zw.start_file("instance.cfg", opts)?;
                                zw.write_all(cfg.as_bytes())?;
                                "minecraft/"
                            } else {
                                let mut loaders = vec![];
                                if let Some(l) = &cf_loader {
                                    loaders.push(serde_json::json!({"id": l, "primary": true}));
                                }
                                let manifest = serde_json::json!({
                                    "minecraft": {"version": mc, "modLoaders": loaders},
                                    "manifestType": "minecraftModpack",
                                    "manifestVersion": 1,
                                    "name": name,
                                    "version": version,
                                    "author": "",
                                    "files": [],
                                    "overrides": "overrides"
                                });
                                zw.start_file("manifest.json", opts)?;
                                zw.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
                                "overrides/"
                            };
							if let Some(ref icon_path) = icon_path {
								let entry = if format == ExportFormat::Prism { "icon.png" } else { "overrides/icon.png" };
								zw.start_file(entry, opts)?;
								let mut file = std::fs::File::open(icon_path)?;
								std::io::copy(&mut file, &mut zw)?;
							}
                            for rel in &include {
								if icon_path.is_some() && rel == "icon.png" { continue; }
                                if base.join(rel).exists() {
                                    zip_add(&mut zw, &base, rel, prefix)?;
                                }
                            }
                            zw.finish().map_err(std::io::Error::other)?;
                            std::fs::rename(&tmp, &out2)?;
                            Ok(())
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r.map_err(|e| e.to_string()));
                        if done.is_err() {
                            let _ = std::fs::remove_file(&tmp);
                        }
                        done?;
                    }
                }
                Ok::<_, String>(Some(out))
            }
            .await;
            let _ = tx.send(Msg::ExportDone(res));
        });
    }
}

// ───────────────────────── интерфейс ─────────────────────────

fn opt_text(ui: &mut Ui, on_label: &str, v: &mut Option<String>, hint: &str, multiline: bool) -> bool {
    let mut on = v.is_some();
    let mut changed = false;
    ui.horizontal(|ui| {
        if toggle(ui, &mut on).changed() {
            *v = if on { Some(String::new()) } else { None };
            changed = true;
        }
        label(ui, on_label, medium(13.5), TEXT());
    });
    if let Some(t) = v.as_mut() {
        let te = if multiline { egui::TextEdit::multiline(t).desired_rows(3) } else { egui::TextEdit::singleline(t) };
        changed |= ui.add(te.hint_text(hint).desired_width(f32::INFINITY).font(mono(12.5)).margin(Margin::symmetric(10, 8))).changed();
    }
    changed
}

impl App {
    pub fn open_inst_settings(&mut self, id: &str) {
        self.inst_settings = Some(SettingsDialog { id: id.to_string(), s: None, section: 0 });
        self.be.load_inst_settings(id.to_string());
    }

    pub fn open_export(&mut self, id: &str, format: ExportFormat) {
        self.export = Some(ExportDialog { id: id.to_string(), format, version: "1.0.0".into(), entries: None, busy: false });
        self.be.list_export_root(id.to_string());
    }

    pub fn inst_dialogs(&mut self, ctx: &egui::Context) {
        let frame = egui::Frame::new().fill(PANEL()).corner_radius(rr(14)).stroke(Stroke::new(1.0, BORDER())).inner_margin(Margin::same(22));
        let screen = ctx.content_rect();

        // ── настройки сборки
        if let Some(dlg) = self.inst_settings.as_mut() {
            let mut close = false;
            let mut save = None;
            let r = egui::Modal::new(Id::new("inst-settings")).frame(frame).show(ctx, |ui| {
                ui.set_width(600.0);
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.horizontal(|ui| {
                    label(ui, "Настройки сборки", bold(20.0), TEXT());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, Icon::Close, 30.0, "Закрыть").clicked() {
                            close = true;
                        }
                    });
                });
                let Some(s) = dlg.s.as_mut() else {
                    label(ui, "Загружаю…", regular(13.5), TEXT_DIM());
                    return;
                };
                tabs(ui, Id::new("is-tabs"), &["Общее".to_string(), "Java и память".into(), "Окно игры".into(), "Хуки".into()], &mut dlg.section);
                egui::ScrollArea::vertical().max_height((screen.height() * 0.55).max(260.0)).auto_shrink([false, true]).show(ui, |ui| {
                    ui.set_width(ui.available_width() - 8.0);
                    ui.spacing_mut().item_spacing.y = 10.0;
                    match dlg.section {
                        0 => {
                            label(ui, "Название", semibold(13.0), TEXT_DIM());
                            ui.add(egui::TextEdit::singleline(&mut s.name).desired_width(f32::INFINITY).font(regular(14.0)).margin(Margin::symmetric(10, 8)));
                            label(ui, format!("Minecraft {} · {} {}", s.game_version, s.loader, s.loader_version), regular(13.5), TEXT_DIM());
                            label(ui, "Аватарку можно сменить кликом по иконке сборки.", regular(12.5), TEXT_FAINT());
                        }
                        1 => {
                            let mut on = s.memory_mb.is_some();
                            ui.horizontal(|ui| {
                                if toggle(ui, &mut on).changed() {
                                    s.memory_mb = if on { Some(s.global_memory_mb) } else { None };
                                }
                                label(ui, "Своя память для этой сборки", medium(13.5), TEXT());
                            });
                            match s.memory_mb.as_mut() {
                                Some(mb) => {
                                    let mut gb = *mb as f32 / 1024.0;
                                    if ui.add(egui::Slider::new(&mut gb, 1.0..=24.0).step_by(0.5).fixed_decimals(1).suffix(" ГБ")).changed() {
                                        *mb = (gb * 1024.0) as u32;
                                    }
                                }
                                None => {
                                    label(ui, format!("Общая настройка: {:.1} ГБ", s.global_memory_mb as f32 / 1024.0), regular(12.5), TEXT_FAINT());
                                }
                            }
                            ui.add_space(4.0);
                            let hint = if s.global_args.is_empty() { "-XX:+UseZGC …".to_string() } else { s.global_args.clone() };
                            opt_text(ui, "Свои аргументы Java", &mut s.java_args, &hint, false);
                            label(ui, "Флаги G1/ZGC из «Производительности» добавляются сами; свои аргументы важнее.", regular(12.5), TEXT_FAINT());
                            opt_text(ui, "Своя Java (путь к java)", &mut s.java_path, "/usr/lib/jvm/java-21-openjdk/bin/java", false);
                            opt_text(ui, "Переменные окружения", &mut s.env_vars, "ИМЯ=значение, по одной на строку", true);
                        }
                        2 => {
                            let mut on = s.fullscreen.is_some();
                            ui.horizontal(|ui| {
                                if toggle(ui, &mut on).changed() {
                                    s.fullscreen = if on { Some(true) } else { None };
                                }
                                label(ui, "Своя настройка полноэкранного режима", medium(13.5), TEXT());
                            });
                            if let Some(fs) = s.fullscreen.as_mut() {
                                checkbox(ui, fs, "Запускать на весь экран");
                            }
                            let mut on = s.resolution.is_some();
                            ui.horizontal(|ui| {
                                if toggle(ui, &mut on).changed() {
                                    s.resolution = if on { Some(s.global_res) } else { None };
                                }
                                label(ui, "Свой размер окна", medium(13.5), TEXT());
                            });
                            match s.resolution.as_mut() {
                                Some((w, h)) => {
                                    ui.horizontal(|ui| {
                                        ui.add(egui::DragValue::new(w).range(320..=7680).suffix(" px"));
                                        label(ui, "×", regular(14.0), TEXT_DIM());
                                        ui.add(egui::DragValue::new(h).range(240..=4320).suffix(" px"));
                                    });
                                }
                                None => {
                                    label(ui, format!("Общая настройка: {} × {}", s.global_res.0, s.global_res.1), regular(12.5), TEXT_FAINT());
                                }
                            }
                        }
                        _ => {
                            label(ui, "Команды, которые выполняются вокруг запуска игры (как в Prism).", regular(13.0), TEXT_DIM());
                            for (title, v, hint) in [
                                ("Перед запуском", &mut s.pre_launch, "notify-send 'Запускаю'"),
                                ("Обёртка", &mut s.wrapper, "prime-run / obs-gamecapture"),
                                ("После выхода", &mut s.post_exit, "~/скрипты/бэкап-миров.sh"),
                            ] {
                                label(ui, title, semibold(13.0), TEXT_DIM());
                                ui.add(egui::TextEdit::singleline(v).hint_text(hint).desired_width(f32::INFINITY).font(mono(12.5)).margin(Margin::symmetric(10, 8)));
                            }
                            label(ui, "GameMode и MangoHud включаются в Настройки → Производительность.", regular(12.5), TEXT_FAINT());
                        }
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if button(ui, "Сохранить", None, Kind::Primary, 36.0).clicked() {
                        save = Some(s.clone());
                    }
                    if button(ui, "Отмена", None, Kind::Secondary, 36.0).clicked() {
                        close = true;
                    }
                });
            });
            if let Some(s) = save {
                self.be.save_inst_settings(dlg.id.clone(), s);
                close = true;
            }
            if close || r.should_close() {
                self.inst_settings = None;
            }
        }

        // ── экспорт
        if let Some(dlg) = self.export.as_mut() {
            let mut close = false;
            let mut go = None;
            let r = egui::Modal::new(Id::new("export")).frame(frame).show(ctx, |ui| {
                ui.set_width(520.0);
                ui.spacing_mut().item_spacing.y = 10.0;
                label(ui, "Экспорт сборки", bold(20.0), TEXT());
                let mut fi = match dlg.format {
                    ExportFormat::Prism => 0,
                    ExportFormat::Modrinth => 1,
                    ExportFormat::CurseForge => 2,
                };
                if segmented(ui, Id::new("exp-fmt"), &["Prism (zip)", "Modrinth (mrpack)", "CurseForge (zip)"], &mut fi) {
                    dlg.format = [ExportFormat::Prism, ExportFormat::Modrinth, ExportFormat::CurseForge][fi];
                }
                let note = match dlg.format {
                    ExportFormat::Prism => "Импорт в Prism/MultiMC: «Добавить сборку → Импорт». Загрузчик и версия подтянутся сами.",
                    ExportFormat::Modrinth => "Моды с Modrinth попадут ссылками (архив маленький), остальное — файлами.",
                    ExportFormat::CurseForge => "Моды кладутся файлами в overrides — CurseForge App и Prism такое открывают.",
                };
                label(ui, note, regular(12.5), TEXT_DIM());
                ui.horizontal(|ui| {
                    label(ui, "Версия сборки", semibold(13.0), TEXT_DIM());
                    ui.add(egui::TextEdit::singleline(&mut dlg.version).desired_width(120.0).margin(Margin::symmetric(10, 6)));
                });
                label(ui, "Что положить в архив", semibold(13.0), TEXT_DIM());
                match dlg.entries.as_mut() {
                    None => {
                        label(ui, "Считаю файлы…", regular(13.0), TEXT_DIM());
                    }
                    Some(entries) => {
                        egui::Frame::new().fill(INPUT()).corner_radius(rr(10)).inner_margin(10).show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height((screen.height() * 0.4).max(180.0)).auto_shrink([false, true]).show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                for e in entries.iter_mut() {
                                    ui.horizontal(|ui| {
                                        let name = if e.dir { format!("{}/", e.name) } else { e.name.clone() };
                                        checkbox(ui, &mut e.include, &name);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.add_space(6.0);
                                            label(ui, human(e.size), mono(12.0), TEXT_FAINT());
                                        });
                                    });
                                }
                            });
                        });
                        let total: u64 = entries.iter().filter(|e| e.include).map(|e| e.size).sum();
                        label(ui, format!("Выбрано: {}", human(total)), regular(12.5), TEXT_DIM());
                    }
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let ready = dlg.entries.is_some() && !dlg.busy;
                    ui.add_enabled_ui(ready, |ui| {
                        if button(ui, if dlg.busy { "Экспортирую…" } else { "Экспортировать" }, Some(Icon::Download), Kind::Primary, 36.0).clicked() {
                            let inc: Vec<String> = dlg.entries.iter().flatten().filter(|e| e.include).map(|e| e.name.clone()).collect();
                            go = Some(inc);
                        }
                    });
                    if button(ui, "Отмена", None, Kind::Secondary, 36.0).clicked() {
                        close = true;
                    }
                });
            });
            if let Some(inc) = go {
                dlg.busy = true;
                let v = if dlg.version.trim().is_empty() { "1.0.0".to_string() } else { dlg.version.trim().to_string() };
                self.be.export_instance(dlg.id.clone(), dlg.format, v, inc);
            }
            if (close || r.should_close()) && !dlg.busy {
                self.export = None;
            }
        }
    }
}
