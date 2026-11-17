use crate::app::*;
use theseus::prelude::JvmPreset;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align, Align2, Frame, Id, Layout, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

const SECTIONS: [(Icon, &str); 6] = [
    (Icon::Image, "Внешний вид"),
    (Icon::Cube, "Производительность"),
    (Icon::Play, "Java"),
    (Icon::Folder, "Папки"),
    (Icon::User, "Аккаунты"),
    (Icon::Globe, "О лаунчере"),
];


impl App {
    pub fn page_settings(&mut self, ui: &mut Ui) {
        self.settings_section = self.settings_section.min(SECTIONS.len() - 1);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(214.0, 10.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(214.0);
                label(ui, "Настройки", bold(26.0), TEXT());
                ui.add_space(14.0);
                ui.spacing_mut().item_spacing.y = 2.0;
                for (i, (ic, name)) in SECTIONS.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(vec2(214.0, 36.0), Sense::click());
                    let t = hover_anim(ui, resp.id, resp.hovered());
                    let active = self.settings_section == i;
                    ui.painter().rect_filled(r, rr(8), HOVER().gamma_multiply(if active { 0.8 } else { t * 0.6 }));
                    let col = if active { TEXT() } else { TEXT_DIM().lerp_to_gamma(TEXT(), t) };
                    icon(ui.painter(), *ic, Rect::from_center_size(pos2(r.left() + 20.0, r.center().y), Vec2::splat(17.0)), if active { accent() } else { col });
                    text_at(ui.painter(), pos2(r.left() + 40.0, r.center().y), Align2::LEFT_CENTER, *name, if active { semibold(14.0) } else { medium(14.0) }, col);
                    if resp.clicked() {
                        self.settings_section = i;
                        match i {
                            2 => self.be.load_java(),
                            3 => self.be.current_launcher_dir(),
                            ACCOUNTS_SECTION => self.be.load_users(),
                            _ => {}
                        }
                    }
                }
            });
            ui.add_space(36.0);
            ui.vertical(|ui| {
                ui.set_max_width(700.0);
                label(ui, SECTIONS[self.settings_section].1, bold(20.0), TEXT());
                ui.add_space(10.0);
                match self.settings_section {
                    0 => self.set_appearance(ui),
                    1 => self.set_perf(ui),
                    2 => self.set_java(ui),
                    3 => self.set_folders(ui),
                    ACCOUNTS_SECTION => self.set_accounts(ui),
                    _ => self.set_about(ui),
                }
            });
        });
    }

    fn set_appearance(&mut self, ui: &mut Ui) {
        crate::look::FONTS.start(ui.ctx());
        if self.icon_packs.is_none() {
            let ctx = ui.ctx().clone();
            self.icon_packs = Some(crate::look::pack_names().into_iter().map(|n| (n.clone(), crate::look::icon_style(&ctx, &format!("pack:{n}"), self.prefs.icon_tint))).collect());
        }

        // ── Макет ──
        label(ui, "Макет", semibold(14.5), TEXT());
        label(ui, "Облик целиком: тема, где меню, углы, плотность, иконки и шрифт. Ниже всё можно подправить по отдельности.", regular(13.0), TEXT_DIM());
        ui.add_space(6.0);
        let mut picked_preset = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for (i, pr) in crate::look::PRESETS.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(158.0, 124.0), Sense::click());
                let t = hover_anim(ui, resp.id, resp.hovered());
                let sel = ui.ctx().animate_bool_with_time(Id::new("preset-sel").with(i), self.prefs.preset == pr.id, 0.15);
                preset_preview(ui.painter(), r, pr, t, sel);
                if resp.on_hover_text(pr.desc).clicked() {
                    picked_preset = Some(pr);
                }
            }
        });
        if let Some(pr) = picked_preset {
            if !crate::look::apply_preset(&mut self.prefs, pr) {
                self.font_pending = true;
            }
            self.apply_look(ui.ctx());
            self.toast(ui, format!("Макет «{}»", pr.name), Icon::Image);
        }
        ui.add_space(8.0);
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, BORDER()));
        ui.add_space(10.0);

        // ── Тема ──
        label(ui, "Тема", semibold(14.5), TEXT());
        label(ui, format!("{} тем: тёмные и светлые. Меняется сразу.", THEMES.len()), regular(13.0), TEXT_DIM());
        ui.add_space(6.0);
        let cur = theme_index(&self.prefs.theme);
        let mut picked = None;
        let all_id = Id::new("themes-all");
        let mut all = ui.data(|d| d.get_temp::<bool>(all_id)).unwrap_or(false);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for (i, th) in THEMES.iter().enumerate() {
                // свёрнуто: первые 7 и выбранная
                if !all && i >= 7 && i != cur {
                    continue;
                }
                let (r, resp) = ui.allocate_exact_size(vec2(158.0, 112.0), Sense::click());
                let t = hover_anim(ui, resp.id, resp.hovered());
                let sel = ui.ctx().animate_bool_with_time(Id::new("theme-sel").with(i), cur == i, 0.15);
                theme_preview(ui.painter(), r, th, t, sel);
                if resp.on_hover_text(th.name).clicked() && cur != i {
                    picked = Some(i);
                }
            }
        });
        if button(ui, if all { "Свернуть".to_string() } else { format!("Все темы ({})", THEMES.len()) }.as_str(), None, Kind::Secondary, 30.0).clicked() {
            all = !all;
            ui.data_mut(|d| d.insert_temp(all_id, all));
        }
        if let Some(i) = picked {
            self.prefs.theme = THEMES[i].id.to_string();
            self.apply_look(ui.ctx());
        }
        ui.add_space(12.0);

        // ── Акцент ──
        label(ui, "Акцент", semibold(14.5), TEXT());
        label(ui, "Цвет кнопок, подсветок и логотипа. «Как в теме» — свой акцент у каждой темы.", regular(13.0), TEXT_DIM());
        ui.add_space(4.0);
        let theme_acc = hex(THEMES[cur].p.accent);
        let mut acc_pick: Option<Option<usize>> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let opts = std::iter::once((None, "Как в теме", theme_acc)).chain(ACCENTS.iter().enumerate().map(|(i, (n, c))| (Some(i), *n, hex(*c))));
            for (key, name, col) in opts {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                let t = hover_anim(ui, resp.id, resp.hovered());
                let sel = ui.ctx().animate_bool_with_time(Id::new("acc").with(key.map(|k| k as i64).unwrap_or(-1)), self.prefs.accent == key, 0.15);
                let rad = 12.0 + t * 1.5;
                ui.painter().circle_filled(r.center(), rad, col);
                if key.is_none() {
                    // «как в теме» — кружок с буквой Т
                    text_at(ui.painter(), r.center(), Align2::CENTER_CENTER, "Т", bold(12.0), on_color(col));
                }
                if sel > 0.0 {
                    ui.painter().circle_stroke(r.center(), 16.0, Stroke::new(2.0, TEXT().gamma_multiply(sel)));
                }
                if resp.on_hover_text(name).clicked() {
                    acc_pick = Some(key);
                }
            }
        });
        if let Some(k) = acc_pick {
            self.prefs.accent = k;
            self.apply_look(ui.ctx());
        }
        ui.add_space(8.0);
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, BORDER()));
        ui.add_space(8.0);
        self.set_layout_parts(ui);
        ui.add_space(8.0);
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, BORDER()));
        ui.add_space(8.0);

        // ── Котики ──
        let cur_name = if self.prefs.cat == crate::cats::NONE_ID {
            "Без котика".to_string()
        } else {
            self.cats.iter().find(|c| c.id == self.prefs.cat).map(|c| c.name.clone()).unwrap_or_else(|| self.prefs.cat.clone())
        };
        let mut changed = false;
        setting_row(ui, "Набор котиков", "Картинка в углу окна, как в Prism Launcher", |ui| {
            egui::ComboBox::from_id_salt("catpack").selected_text(cur_name).width(300.0).height(420.0).show_ui(ui, |ui| {
                if ui.selectable_label(self.prefs.cat == crate::cats::NONE_ID, "Без котика").clicked() {
                    self.prefs.cat = crate::cats::NONE_ID.into();
                    changed = true;
                }
                for c in &self.cats {
                    let text = if c.origin.is_empty() { c.name.clone() } else { format!("{}  · {}", c.name, c.origin) };
                    if ui.selectable_label(self.prefs.cat == c.id, text).clicked() {
                        self.prefs.cat = c.id.clone();
                        changed = true;
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if button(ui, "Свой котик…", Some(Icon::Plus), Kind::Primary, 32.0).clicked() {
                self.be.pick(crate::backend::Pick::CatImage);
            }
            if button(ui, "Открыть папку", Some(Icon::Folder), Kind::Secondary, 32.0).clicked() {
                let d = crate::cats::own_dir();
                let _ = std::fs::create_dir_all(&d);
                crate::sys::open(&d);
            }
            if button(ui, "Перезагрузить все", Some(Icon::Refresh), Kind::Secondary, 32.0).clicked() {
                self.cats = crate::cats::scan();
                ui.ctx().forget_all_images();
                self.cat_seed = self.cat_seed.wrapping_add(1);
                let n = self.cats.len();
                self.toast(ui, format!("Наборов котиков: {n}"), Icon::Refresh);
            }
        });
        label(ui, "«Свой котик…» — выбрать картинку или GIF. Можно и папкой с картинками в папке котиков; котики из Prism подхватываются сами.", regular(12.5), TEXT_FAINT());
        ui.add_space(6.0);
        if self.prefs.cat != crate::cats::NONE_ID {
            setting_row(ui, "Прозрачность котика", "100 % — непрозрачный", |ui| {
                let mut op = self.prefs.cat_opacity as f32;
                if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).step_by(5.0).fixed_decimals(0).suffix(" %")).changed() {
                    self.prefs.cat_opacity = op as u8;
                    changed = true;
                }
            });
            setting_row(ui, "Размещение", "Как CatFit в Prism", |ui| {
                if segmented(ui, Id::new("cat-fit"), &crate::cats::FITS, &mut self.prefs.cat_fit) {
                    changed = true;
                }
            });
        }
        if changed {
            crate::prefs::save(&self.prefs);
        }
        ui.add_space(8.0);
        ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, BORDER()));
        ui.add_space(8.0);
        setting_row(ui, "Масштаб интерфейса", "Применяется, когда отпустите ползунок", |ui| {
            let mut pct = self.prefs.scale * 100.0;
            let resp = ui.add(egui::Slider::new(&mut pct, 80.0..=150.0).step_by(5.0).fixed_decimals(0).suffix(" %"));
            self.prefs.scale = pct / 100.0;
            if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                ui.ctx().set_zoom_factor(self.prefs.scale);
                crate::prefs::save(&self.prefs);
            }
        });
        setting_row(ui, "Анимации", "Плавные переходы между страницами", |ui| {
            if toggle(ui, &mut self.prefs.animations).changed() {
                crate::prefs::save(&self.prefs);
            }
        });
    }

    /// Меню, форма, иконки и шрифт.
    fn set_layout_parts(&mut self, ui: &mut Ui) {
        use crate::look::NAVS;
        let mut changed = false;
        setting_row(ui, "Меню", "Где кнопки разделов: узкая панель, панель с подписями, тулбар сверху или док снизу", |ui| {
            let mut i = NAVS.iter().position(|(n, _)| *n == self.prefs.nav).unwrap_or(0);
            let labels: Vec<&str> = NAVS.iter().map(|(_, l)| *l).collect();
            if segmented(ui, Id::new("nav-pos"), &labels, &mut i) {
                self.prefs.nav = NAVS[i].0;
                changed = true;
            }
        });
        setting_row(ui, "Скругление углов", "0 % — квадратные, как в Prism и Minecraft", |ui| {
            let mut pct = self.prefs.round * 100.0;
            let resp = ui.add(egui::Slider::new(&mut pct, 0.0..=200.0).step_by(5.0).fixed_decimals(0).suffix(" %"));
            if resp.changed() {
                self.prefs.round = pct / 100.0;
                changed = true;
            }
        });
        setting_row(ui, "Плотность", "Отступы между элементами", |ui| {
            const D: [f32; 3] = [0.8, 1.0, 1.2];
            let mut i = D.iter().enumerate().min_by(|a, b| (a.1 - self.prefs.density).abs().total_cmp(&(b.1 - self.prefs.density).abs())).map(|(i, _)| i).unwrap_or(1);
            if segmented(ui, Id::new("density"), &["Компактно", "Обычно", "Просторно"], &mut i) {
                self.prefs.density = D[i];
                changed = true;
            }
        });
        ui.add_space(8.0);

        // ── Иконки ──
        label(ui, "Иконки", semibold(14.5), TEXT());
        label(ui, "Встроенные стили или свой набор картинок (PNG/SVG) из папки иконок.", regular(13.0), TEXT_DIM());
        ui.add_space(6.0);
        let mut styles: Vec<(String, String, IconStyle)> = crate::look::ICON_STYLES.iter().map(|(id, name)| (id.to_string(), name.to_string(), crate::look::icon_style(ui.ctx(), id, true))).collect();
        for (n, st) in self.icon_packs.iter().flatten() {
            styles.push((format!("pack:{n}"), n.clone(), st.clone()));
        }
        let mut pick_icons = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for (id, name, st) in &styles {
                let (r, resp) = ui.allocate_exact_size(vec2(158.0, 66.0), Sense::click());
                let t = hover_anim(ui, resp.id, resp.hovered());
                let sel = self.prefs.icons == *id;
                let p = ui.painter();
                p.rect(r, rr(10), PANEL(), Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { accent() } else { BORDER().lerp_to_gamma(TEXT_DIM(), t) }), StrokeKind::Inside);
                for (k, ic) in [Icon::Home, Icon::Library, Icon::Browse, Icon::Skins, Icon::Settings].into_iter().enumerate() {
                    let c = pos2(r.left() + 22.0 + k as f32 * 28.5, r.top() + 22.0);
                    icon_styled(p, ic, Rect::from_center_size(c, Vec2::splat(20.0)), if k == 0 { accent() } else { TEXT_DIM().lerp_to_gamma(TEXT(), t) }, st);
                }
                text_at(p, pos2(r.left() + 12.0, r.bottom() - 15.0), Align2::LEFT_CENTER, name, if sel { semibold(13.0) } else { medium(13.0) }, if sel { TEXT() } else { TEXT_DIM() });
                if resp.clicked() && !sel {
                    pick_icons = Some(id.clone());
                }
            }
        });
        if let Some(id) = pick_icons {
            self.prefs.icons = id;
            changed = true;
        }
        ui.add_space(4.0);
        let mut reload = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if button(ui, "Создать свой набор", Some(Icon::Plus), Kind::Primary, 32.0).on_hover_text("Папка со встроенными иконками в SVG — замените их своими").clicked() {
                match crate::look::make_template() {
                    Ok(dir) => {
                        crate::sys::open(&dir);
                        reload = true;
                    }
                    Err(e) => self.toast(ui, format!("Не удалось: {e}"), Icon::Close),
                }
            }
            if button(ui, "Папка иконок", Some(Icon::Folder), Kind::Secondary, 32.0).clicked() {
                let d = crate::look::icons_dir();
                let _ = std::fs::create_dir_all(&d);
                crate::sys::open(&d);
            }
            if button(ui, "Обновить", Some(Icon::Refresh), Kind::Secondary, 32.0).on_hover_text("Перечитать свои наборы после правок").clicked() {
                reload = true;
            }
        });
        if self.prefs.icons.starts_with("pack:") || self.icon_packs.as_ref().is_some_and(|p| !p.is_empty()) {
            setting_row(ui, "Красить свои иконки в цвет темы", "Выключите для цветных картинок", |ui| {
                if toggle(ui, &mut self.prefs.icon_tint).changed() {
                    reload = true;
                }
            });
        }
        if reload {
            self.icon_packs = None;
            crate::look::reload_icons(ui.ctx(), &self.prefs);
            crate::prefs::save(&self.prefs);
        }
        ui.add_space(8.0);

        // ── Шрифт ──
        label(ui, "Шрифт", semibold(14.5), TEXT());
        label(ui, "Встроенные, любой шрифт с кириллицей из системы или свой файл TTF/OTF.", regular(13.0), TEXT_DIM());
        ui.add_space(6.0);
        let mut pick_font: Option<(String, Vec<String>)> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let fams = crate::look::FONTS.get();
            egui::ComboBox::from_id_salt("font").selected_text(crate::look::font_label(&self.prefs.font)).width(300.0).height(420.0).show_ui(ui, |ui| {
                for (key, name) in [(crate::look::FONT_INTER, "Inter"), (crate::look::FONT_PIXEL, "Pixelify Sans — пиксельный"), (crate::look::FONT_MONO, "Hack — моноширинный")] {
                    if ui.selectable_label(self.prefs.font == key, format!("{name}  · встроенный")).clicked() {
                        pick_font = Some((key.into(), vec![]));
                    }
                }
                ui.separator();
                match &fams {
                    None => {
                        ui.label("Ищу шрифты в системе…");
                    }
                    Some(list) if list.is_empty() => {
                        ui.label("В системе не нашлось шрифтов с кириллицей");
                    }
                    Some(list) => {
                        for f in list.iter() {
                            let key = format!("sys:{}", f.name);
                            if ui.selectable_label(self.prefs.font == key, &f.name).clicked() {
                                pick_font = Some((key, f.files()));
                            }
                        }
                    }
                }
            });
            if button(ui, "Свой файл…", Some(Icon::Plus), Kind::Secondary, 32.0).clicked() {
                self.be.pick(crate::backend::Pick::FontFile);
            }
        });
        if let Some((f, files)) = pick_font {
            self.prefs.font = f;
            self.prefs.font_files = files;
            changed = true;
        }
        ui.add_space(4.0);
        Frame::new().fill(PANEL()).corner_radius(rr(10)).stroke(Stroke::new(1.0, BORDER())).inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            label(ui, "Играть на TrollLand", bold(20.0), TEXT());
            label(ui, "Съешь же ещё этих мягких французских булок, да выпей чаю. 0123456789", regular(14.0), TEXT());
            label(ui, "Сборки · Моды · Скины — полужирный и средний", semibold(13.0), TEXT_DIM());
        });
        if changed {
            self.apply_look(ui.ctx());
        }
    }

    fn set_perf(&mut self, ui: &mut Ui) {
        setting_row(ui, "Память для игры", "Общая настройка для всех сборок (как в TrollLauncher)", |ui| {
            let resp = ui.add(egui::Slider::new(&mut self.s.ram, 1.0..=16.0).step_by(0.5).fixed_decimals(1).suffix(" ГБ"));
            if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                self.be.set_memory((self.s.ram * 1024.0) as u32);
            }
        });
        let Some(mut p) = self.perf.clone() else {
            self.be.load_perf();
            label(ui, "Загружаю…", regular(13.0), TEXT_DIM());
            return;
        };
        let mut changed = false;

        ui.add_space(10.0);
        section_title(ui, "Игра");
        setting_row(ui, "Флаги Java", "G1 — флаги Aikar, как в сборке TrollLand для Prism. ZGC — почти без пауз сборщика мусора (Java 17+). Свои аргументы сборки всё равно важнее.", |ui| {
            let mut i = match p.s.jvm_preset {
                JvmPreset::Off => 0,
                JvmPreset::G1 => 1,
                JvmPreset::Zgc => 2,
            };
            if segmented(ui, Id::new("jvm-preset"), &["Выкл", "G1", "ZGC"], &mut i) {
                p.s.jvm_preset = [JvmPreset::Off, JvmPreset::G1, JvmPreset::Zgc][i];
                changed = true;
            }
        });
        setting_row(ui, "Вся память сразу", "-Xms = -Xmx: Java не растягивает кучу во время игры (меньше фризов, но память занята целиком)", |ui| {
            changed |= toggle(ui, &mut p.s.full_heap_at_start).changed();
        });
        if cfg!(target_os = "linux") {
            let gm = if p.tools.gamemode { "Feral GameMode: режим производительности CPU и приоритет игре" } else { "gamemoderun не найден — поставьте пакет gamemode" };
            setting_row(ui, "GameMode", gm, |ui| {
                ui.add_enabled_ui(p.tools.gamemode, |ui| changed |= toggle(ui, &mut p.s.gamemode).changed());
            });
            let mh = if p.tools.mangohud { "Счётчик FPS, загрузка CPU/GPU поверх игры" } else { "mangohud не найден — поставьте пакет mangohud" };
            setting_row(ui, "MangoHud", mh, |ui| {
                ui.add_enabled_ui(p.tools.mangohud, |ui| changed |= toggle(ui, &mut p.s.mangohud).changed());
            });
        }
        setting_row(ui, "Автообновление sfatm", "Перед запуском сборки с нашим модом sfatm лаунчер скачивает новую версию с GitHub (sfatm-releases), если она вышла", |ui| {
            changed |= toggle(ui, &mut p.s.sfatm_autoupdate).changed();
        });

        ui.add_space(10.0);
        section_title(ui, "Лаунчер во время игры");
        setting_row(ui, "Тихий режим", "Пока идёт игра, лаунчер не анимируется и почти не перерисовывается — не отнимает у игры CPU и видеокарту", |ui| {
            changed |= toggle(ui, &mut p.s.pause_ui_while_playing).changed();
        });
        setting_row(ui, "Сворачивать при запуске игры", "Как в Prism Launcher. В niri сворачивания нет — там спасает тихий режим", |ui| {
            changed |= toggle(ui, &mut p.hide_on_start).changed();
        });
        setting_row(ui, "Возвращать окно после игры", "Развернуть лаунчер, когда игра закрылась", |ui| {
            changed |= toggle(ui, &mut p.refocus_on_close).changed();
        });
        if changed {
            self.be.save_perf(p.clone());
            self.perf = Some(p);
        }
    }

    fn set_java(&mut self, ui: &mut Ui) {
        label(ui, "Какой Java запускать игру. Новым версиям Minecraft нужна Java 21, 1.17–1.20.4 — Java 17, старым — Java 8. Если Java не выбрана, лаунчер скачает её сам при запуске.", regular(13.0), TEXT_DIM());
        ui.add_space(8.0);
        let Some((cur, found)) = self.java.clone() else {
            self.be.load_java();
            label(ui, "Ищу Java на компьютере…", regular(13.0), TEXT_DIM());
            return;
        };
        for major in [21u32, 17, 8] {
            let current = cur.iter().find(|(m, _, _)| *m == major);
            let desc = match current {
                Some((_, path, ver)) => format!("{ver} · {path}"),
                None => "Не выбрана — скачается сама при первом запуске".to_string(),
            };
            setting_row(ui, &format!("Java {major}"), &desc, |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if button(ui, "Файл…", Some(Icon::Folder), Kind::Secondary, 32.0).on_hover_text("Указать java вручную").clicked() {
                    self.be.pick_java(major);
                }
                if button(ui, "Скачать", Some(Icon::Download), Kind::Secondary, 32.0).on_hover_text("Скачать Java от Adoptium/Azul").clicked() {
                    self.be.install_java(major);
                }
                let options: Vec<&(u32, String, String)> = found.iter().filter(|(m, _, _)| *m == major).collect();
                if !options.is_empty() {
                    let sel = current.map(|(_, p, _)| p.clone()).unwrap_or_else(|| "выбрать…".into());
                    let short = |p: &str| if p.chars().count() > 34 { format!("…{}", p.chars().rev().take(33).collect::<Vec<_>>().into_iter().rev().collect::<String>()) } else { p.to_string() };
                    egui::ComboBox::from_id_salt(("java", major)).selected_text(short(&sel)).width(240.0).show_ui(ui, |ui| {
                        for (_, path, ver) in options {
                            if ui.selectable_label(current.is_some_and(|(_, p, _)| p == path), format!("{ver} — {path}")).clicked() {
                                self.be.set_java(major, std::path::PathBuf::from(path));
                            }
                        }
                    });
                }
            });
        }
        ui.add_space(6.0);
        if button(ui, "Найти Java заново", Some(Icon::Refresh), Kind::Secondary, 32.0).clicked() {
            self.java = None;
        }
        label(ui, "Для отдельной сборки Java можно выбрать в её настройках (меню «⋯» → «Настройки сборки» → «Java и память»).", regular(12.5), TEXT_FAINT());
    }

    fn set_folders(&mut self, ui: &mut Ui) {
        let base = self.launcher_dir.clone().map(std::path::PathBuf::from).unwrap_or_else(dirs_base);
        setting_row(ui, "Папка лаунчера", &base.display().to_string(), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if button(ui, "Сменить…", Some(Icon::Folder), Kind::Primary, 32.0).on_hover_text("Перенести сборки, миры и настройки в другую папку").clicked() {
                self.be.pick_launcher_dir();
            }
            if button(ui, "Открыть", Some(Icon::Folder), Kind::Secondary, 32.0).clicked() {
                crate::sys::open(&base);
            }
        });
        if let Some(p) = self.dir_pending.clone() {
            egui::Frame::new().fill(accent().gamma_multiply(0.12)).corner_radius(rr(10)).inner_margin(12).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(egui::Label::new(egui::RichText::new(format!("Папка поменяется на {} при следующем запуске: лаунчер перенесёт туда сборки, миры, скины и настройки. Это может занять пару минут.", p.display())).font(regular(13.0)).color(TEXT())).wrap());
                ui.add_space(6.0);
                if button(ui, "Перезапустить сейчас", Some(Icon::Refresh), Kind::Primary, 32.0).clicked() {
                    crate::update::restart();
                }
            });
        }
        let inst = base.join("profiles");
        setting_row(ui, "Сборки", &inst.display().to_string(), |ui| {
            if button(ui, "Открыть", Some(Icon::Folder), Kind::Secondary, 32.0).clicked() {
                crate::sys::open(&inst);
            }
        });
    }

    fn set_accounts(&mut self, ui: &mut Ui) {
        label(ui, "Лицензия Microsoft — для официальных серверов и скинов Mojang. Ely.by — бесплатные аккаунты со своими скинами.", regular(13.0), TEXT_DIM());
        ui.add_space(8.0);
        let def = self.default_account().map(|a| a.id);
        let accounts = self.accounts.clone();
        if accounts.is_empty() {
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                label(ui, "Аккаунтов нет", semibold(15.0), TEXT());
                label(ui, "Без входа игра не запустится.", regular(13.0), TEXT_DIM());
            });
        }
        for a in &accounts {
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::click());
            let t = hover_anim(ui, resp.id, resp.hovered());
            let p = ui.painter().clone();
            let active = def == Some(a.id);
            p.rect(r, rr(12), PANEL().lerp_to_gamma(RAISED(), t), Stroke::new(1.0, if active { accent().gamma_multiply(0.6) } else { BORDER() }), StrokeKind::Inside);
            let hr = Rect::from_center_size(pos2(r.left() + 34.0, r.center().y), Vec2::splat(36.0));
            match self.equipped_skin_of(a).and_then(|id| self.skin_tex.get(&id)) {
                Some(tx) => crate::skin3d::draw_face(&p, hr, tx.tex.id(), tx.h),
                None => head(&p, hr, &DEFAULT_LOOK, 7),
            }
            let nr = text_at(&p, pos2(hr.right() + 14.0, r.center().y - 9.0), Align2::LEFT_CENTER, &a.name, semibold(15.0), TEXT());
            let (kind, col) = if a.elyby { ("Ely.by", GREEN()) } else { ("Microsoft", BLUE()) };
            let g = p.layout_no_wrap(kind.to_string(), medium(11.5), col);
            let cr = Rect::from_min_size(pos2(nr.right() + 8.0, r.center().y - 19.0), vec2(g.size().x + 12.0, 20.0));
            p.rect_filled(cr, rr(5), col.gamma_multiply(0.15));
            p.galley(cr.center() - g.size() / 2.0, g, col);
            text_at(&p, pos2(hr.right() + 14.0, r.center().y + 11.0), Align2::LEFT_CENTER, if active { "Используется для запуска" } else { "Нажмите, чтобы выбрать" }, regular(12.5), TEXT_DIM());
            let xr = Rect::from_center_size(pos2(r.right() - 26.0, r.center().y), Vec2::splat(30.0));
            let x = icon_button_at(ui, xr, resp.id.with("rm"), Icon::Close, "Выйти из аккаунта");
            if active {
                icon(&p, Icon::Check, Rect::from_center_size(pos2(r.right() - 64.0, r.center().y), Vec2::splat(18.0)), accent());
            }
            if x.clicked() {
                self.be.remove_user(a.id);
            } else if resp.clicked() && !active {
                self.be.set_default_user(a.id);
            }
            ui.add_space(2.0);
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.add_enabled_ui(!self.login_busy, |ui| {
                if button(ui, if self.login_busy { "Вход…" } else { "Войти через Microsoft" }, Some(Icon::Plus), Kind::Primary, 36.0).clicked() {
                    self.be.microsoft_login();
                }
                if button(ui, "Войти через Ely.by", Some(Icon::Plus), Kind::Secondary, 36.0).clicked() {
                    self.ely.open = true;
                }
            });
        });
    }

    fn set_about(&mut self, ui: &mut Ui) {
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(56.0, 72.0), Sense::hover());
                logo(ui.painter(), r);
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    label(ui, "TrollLauncher (Rust)", bold(22.0), TEXT());
                    label(ui, "Нативный интерфейс на egui + wgpu поверх движка theseus.", regular(13.5), TEXT_DIM());
                    label(ui, "Сборки, аккаунты и скины общие с TrollLauncher.", regular(13.5), TEXT_DIM());
                });
            });
        });
        ui.add_space(12.0);
        setting_row(ui, "Обновления", &match (&self.update, &self.update_checked) {
            (Some(u), _) => format!("Доступна версия {}", u.version),
            (None, Some(t)) => t.clone(),
            _ => "Лаунчер сам проверяет GitHub при запуске и раз в 6 часов".into(),
        }, |ui| {
            if self.update.is_some() {
                if button(ui, "Обновить", Some(Icon::Download), Kind::Primary, 32.0).clicked() {
                    self.update_dialog = true;
                }
            } else if button(ui, "Проверить", Some(Icon::Refresh), Kind::Secondary, 32.0).clicked() {
                self.be.check_update(true);
            }
        });
        ui.add_space(12.0);
        let stats = [("Версия", env!("CARGO_PKG_VERSION").to_string()), ("Сборок", self.instances.len().to_string()), ("Скинов", self.skins.len().to_string())];
        ui.columns(3, |c| {
            for (i, (k, v)) in stats.iter().enumerate() {
                card_frame().show(&mut c[i], |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 2.0;
                    label(ui, v.clone(), bold(22.0), TEXT());
                    label(ui, *k, regular(12.5), TEXT_DIM());
                });
            }
        });
    }
}

fn dirs_base() -> std::path::PathBuf {
    crate::sys::data_root().join(crate::backend::APP_ID)
}

/// Мини-окно макета: палитра, где меню, скругление.
fn preset_preview(p: &egui::Painter, r: Rect, pr: &crate::look::Preset, hover: f32, sel: f32) {
    use crate::look::Nav;
    let c = &THEMES[theme_index(pr.theme)].p;
    let k = pr.round;
    let rad = |v: f32| egui::CornerRadius::same((v * k).round() as u8);
    let win = Rect::from_min_size(r.min, vec2(r.width(), r.height() - 24.0));
    p.rect_filled(win, rad(10.0), hex(c.bg));
    let content = match pr.nav {
        Nav::Side => {
            for i in 0..4 {
                p.rect_filled(Rect::from_center_size(pos2(win.left() + 11.0, win.top() + 16.0 + i as f32 * 13.0), Vec2::splat(7.0)), rad(2.0), if i == 0 { hex(c.accent) } else { hex(c.faint) });
            }
            Rect::from_min_max(pos2(win.left() + 22.0, win.top() + 10.0), win.max)
        }
        Nav::Wide => {
            for i in 0..4 {
                let y = win.top() + 16.0 + i as f32 * 12.0;
                p.rect_filled(Rect::from_center_size(pos2(win.left() + 10.0, y), Vec2::splat(6.0)), rad(2.0), if i == 0 { hex(c.accent) } else { hex(c.faint) });
                p.rect_filled(Rect::from_min_size(pos2(win.left() + 16.0, y - 2.0), vec2(22.0, 4.0)), 2, hex(if i == 0 { c.text } else { c.dim }));
            }
            Rect::from_min_max(pos2(win.left() + 46.0, win.top() + 10.0), win.max)
        }
        Nav::Top => {
            for i in 0..4 {
                let x = win.left() + 10.0 + i as f32 * 26.0;
                p.rect_filled(Rect::from_min_size(pos2(x, win.top() + 7.0), vec2(5.0, 5.0)), rad(1.0), if i == 0 { hex(c.accent) } else { hex(c.faint) });
                p.rect_filled(Rect::from_min_size(pos2(x + 7.0, win.top() + 8.0), vec2(14.0, 3.0)), 1, hex(c.dim));
            }
            Rect::from_min_max(pos2(win.left() + 5.0, win.top() + 19.0), win.max - vec2(5.0, 5.0))
        }
        Nav::Dock => {
            let dock = Rect::from_center_size(pos2(win.center().x, win.bottom() - 10.0), vec2(70.0, 13.0));
            p.rect_filled(dock, rad(6.0), hex(c.panel));
            for i in 0..5 {
                p.rect_filled(Rect::from_center_size(pos2(dock.left() + 11.0 + i as f32 * 12.0, dock.center().y), Vec2::splat(6.0)), rad(2.0), if i == 0 { hex(c.accent) } else { hex(c.faint) });
            }
            Rect::from_min_max(pos2(win.left() + 5.0, win.top() + 10.0), pos2(win.right() - 5.0, win.bottom() - 20.0))
        }
    };
    p.rect_filled(content, rad(7.0), hex(c.content));
    let card = content.shrink(8.0);
    p.rect(card, rad(6.0), hex(c.panel), Stroke::new(1.0, hex(c.border)), StrokeKind::Inside);
    p.rect_filled(Rect::from_min_size(card.min + vec2(7.0, 7.0), vec2(card.width() * 0.55, 5.0)), 2, hex(c.text));
    p.rect_filled(Rect::from_min_size(card.min + vec2(7.0, 16.0), vec2(card.width() * 0.75, 3.0)), 1, hex(c.dim));
    let btn = Rect::from_min_size(pos2(card.left() + 7.0, card.bottom() - 15.0), vec2(34.0, 9.0));
    p.rect_filled(btn, rad(4.0), hex(c.accent));
    let ring = if sel > 0.0 { accent().gamma_multiply(sel) } else { BORDER().lerp_to_gamma(TEXT_DIM(), hover) };
    p.rect_stroke(win, rad(10.0), Stroke::new(if sel > 0.0 { 2.0 } else { 1.0 }, ring), StrokeKind::Outside);
    text_at(p, pos2(r.left() + 2.0, r.bottom() - 9.0), Align2::LEFT_CENTER, pr.name, if sel > 0.5 { semibold(13.0) } else { medium(13.0) }, if sel > 0.5 { TEXT() } else { TEXT_DIM().lerp_to_gamma(TEXT(), hover) });
}

/// Мини-окно темы: боковая панель, страница, карточка с текстом и кнопкой акцента.
fn theme_preview(p: &egui::Painter, r: Rect, th: &Theme, hover: f32, sel: f32) {
    let c = &th.p;
    let win = Rect::from_min_size(r.min, vec2(r.width(), r.height() - 24.0));
    p.rect_filled(win, rr(10), hex(c.bg));
    let content = Rect::from_min_max(pos2(win.left() + 22.0, win.top() + 12.0), win.max);
    p.rect_filled(content, egui::CornerRadius { nw: 7, ne: 0, sw: 0, se: 10 }, hex(c.content));
    for k in 0..3 {
        let y = win.top() + 18.0 + k as f32 * 14.0;
        p.circle_filled(pos2(win.left() + 11.0, y), 3.5, if k == 0 { hex(c.accent) } else { hex(c.faint) });
    }
    let card = Rect::from_min_size(content.min + vec2(9.0, 9.0), vec2(content.width() - 18.0, content.height() - 18.0));
    p.rect(card, rr(6), hex(c.panel), Stroke::new(1.0, hex(c.border)), StrokeKind::Inside);
    p.rect_filled(Rect::from_min_size(card.min + vec2(8.0, 9.0), vec2(card.width() * 0.55, 6.0)), rr(3), hex(c.text));
    p.rect_filled(Rect::from_min_size(card.min + vec2(8.0, 20.0), vec2(card.width() * 0.75, 4.0)), rr(2), hex(c.dim));
    let btn = Rect::from_min_size(pos2(card.left() + 8.0, card.bottom() - 18.0), vec2(38.0, 11.0));
    p.rect_filled(btn, rr(4), hex(c.accent));
    p.rect_filled(Rect::from_min_size(pos2(btn.right() + 5.0, btn.top()), vec2(26.0, 11.0)), rr(4), hex(c.raised));
    for (k, col) in [c.green, c.yellow, c.danger].iter().enumerate() {
        p.circle_filled(pos2(card.right() - 10.0 - k as f32 * 9.0, card.bottom() - 12.5), 3.0, hex(*col));
    }
    let ring = if sel > 0.0 { accent().gamma_multiply(sel) } else { BORDER().lerp_to_gamma(TEXT_DIM(), hover) };
    p.rect_stroke(win, rr(10), Stroke::new(if sel > 0.0 { 2.0 } else { 1.0 }, ring), StrokeKind::Outside);
    text_at(p, pos2(r.left() + 2.0, r.bottom() - 9.0), Align2::LEFT_CENTER, th.name, if sel > 0.5 { semibold(13.0) } else { medium(13.0) }, if sel > 0.5 { TEXT() } else { TEXT_DIM().lerp_to_gamma(TEXT(), hover) });
}
