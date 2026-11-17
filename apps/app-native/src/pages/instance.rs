use crate::app::*;
use crate::backend::{ContentRow, Pick};
use crate::instcfg::ExportFormat;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{
    self, Align, Align2, Color32, Id, Layout, Rect, Sense, Stroke, TextFormat, Ui, Vec2, pos2, text::LayoutJob, vec2,
};

impl App {
    pub fn page_instance(&mut self, ui: &mut Ui, id: &str) {
        let Some(inst) = self.inst(id).cloned() else {
            label(ui, "Сборка не найдена", semibold(16.0), TEXT_DIM());
            return;
        };
        let now = Self::now(ui);

        // ── шапка
        let mut open_settings = false;
        let mut open_export: Option<ExportFormat> = None;
        ui.horizontal(|ui| {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(96.0), Sense::click());
            Self::inst_icon(ui, r, &inst, 18.0);
            let t = hover_anim(ui, resp.id, resp.hovered());
            if t > 0.01 {
                ui.painter().rect_filled(r, rr(18), Color32::from_black_alpha((150.0 * t) as u8));
                icon(ui.painter(), Icon::Image, Rect::from_center_size(r.center() - vec2(0.0, 8.0), Vec2::splat(24.0)), Color32::WHITE.gamma_multiply(t));
                text_at(ui.painter(), r.center() + vec2(0.0, 16.0), Align2::CENTER_CENTER, "Сменить", semibold(12.0), Color32::WHITE.gamma_multiply(t));
            }
            let resp = resp.on_hover_text("Поставить фото на аватарку");
            if resp.clicked() {
                self.be.pick(Pick::InstanceIcon(inst.id.clone()));
            }
            egui::Popup::context_menu(&resp).show(|ui| {
                if ui.button("Выбрать фото…").clicked() {
                    self.be.pick(Pick::InstanceIcon(inst.id.clone()));
                }
                if inst.icon.is_some() && ui.button("Убрать фото").clicked() {
                    self.be.reset_icon(inst.id.clone());
                }
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.add_space(6.0);
                label(ui, &inst.name, bold(28.0), TEXT());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    chip(ui, &inst.game_version, RAISED(), TEXT());
                    chip(ui, &inst.loader, RAISED(), TEXT());
                    if let Some(c) = self.content.get(&inst.id) {
                        chip(ui, &format!("{} модов", c.iter().filter(|r| r.kind == 0).count()), RAISED(), TEXT_DIM());
                    }
                    chip(ui, &format!("{} в игре", fmt_played(inst.played_secs)), RAISED(), TEXT_DIM());
                });
                label(ui, format!("Последний запуск: {}", fmt_ago(inst.last_played)), regular(13.0), TEXT_FAINT());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let more = icon_button(ui, Icon::More, 40.0, "Ещё");
                egui::Popup::menu(&more).show(|ui| {
                    ui.set_min_width(200.0);
                    if ui.button("⚙  Настройки сборки").clicked() {
                        open_settings = true;
                    }
                    ui.menu_button("⇪  Экспортировать", |ui| {
                        ui.set_min_width(200.0);
                        for f in [ExportFormat::Prism, ExportFormat::Modrinth, ExportFormat::CurseForge] {
                            if ui.button(f.title()).clicked() {
                                open_export = Some(f);
                            }
                        }
                    });
                    if ui.button("Открыть папку").clicked() {
                        self.be.open_folder(inst.id.clone());
                    }
                    ui.separator();
                    if ui.button("Переименовать").clicked() {
                        self.rename = RenameDialog { open: true, id: inst.id.clone(), name: inst.name.clone() };
                    }
                    if ui.button("Сменить аватарку…").clicked() {
                        self.be.pick(Pick::InstanceIcon(inst.id.clone()));
                    }
                    if inst.icon.is_some() && ui.button("Убрать аватарку").clicked() {
                        self.be.reset_icon(inst.id.clone());
                    }
                    ui.separator();
                    if ui.button(egui::RichText::new("Удалить сборку").color(lighten(DANGER(), 0.2))).clicked() {
                        self.confirm_delete = Some(inst.id.clone());
                    }
                });
                if icon_button(ui, Icon::Folder, 40.0, "Открыть папку").clicked() {
                    self.be.open_folder(inst.id.clone());
                }
                ui.add_space(6.0);
                let resp = if inst.installing {
                    button(ui, "Устанавливается…", Some(Icon::Download), Kind::Secondary, 42.0)
                } else if self.is_running(&inst.id) {
                    button(ui, "Остановить", Some(Icon::Stop), Kind::Danger, 42.0)
                } else if self.is_launching(&inst.id, now) {
                    button(ui, "Запуск…", Some(Icon::Play), Kind::Secondary, 42.0)
                } else {
                    button(ui, "  Играть  ", Some(Icon::Play), Kind::Primary, 42.0)
                };
                if resp.clicked() && !inst.installing {
                    self.play(ui, &inst.id);
                }
            });
        });
        if open_settings {
            self.open_inst_settings(&inst.id);
        }
        if let Some(f) = open_export {
            self.open_export(&inst.id, f);
        }
        ui.add_space(18.0);

        let n = self.content.get(&inst.id).map(|c| c.len()).unwrap_or(0);
        let labels = vec![format!("Контент  {n}"), "Скриншоты".into(), "Журнал".into()];
        if tabs(ui, Id::new("inst-tabs"), &labels, &mut self.inst_tab) {
            match self.inst_tab {
                1 => self.be.load_screens(inst.id.clone()),
                2 => self.be.load_log(inst.id.clone()),
                _ => self.be.load_content(inst.id.clone()),
            }
        }
        let y = ui.cursor().top() - 8.0;
        ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, BORDER()));
        ui.add_space(12.0);

        match self.inst_tab {
            0 => self.tab_content(ui, &inst.id),
            1 => self.tab_screens(ui, &inst.id),
            _ => self.tab_log(ui, &inst.id),
        }
    }

    fn tab_content(&mut self, ui: &mut Ui, id: &str) {
        let rows: Vec<ContentRow> = self.content.get(id).cloned().unwrap_or_default();
        let updates: Vec<String> = rows.iter().filter(|r| r.has_update).map(|r| r.file_path.clone()).collect();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            search_field(ui, &mut self.mod_search, "Поиск по модам", 250.0);
            segmented(ui, Id::new("mod-filter"), &["Всё", "Моды", "Ресурспаки", "Шейдеры", "Датапаки"], &mut self.mod_filter);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if button(ui, "Добавить", Some(Icon::Plus), Kind::Primary, 34.0).clicked() {
                    self.browse_target = Some(id.to_string());
                    self.go(ui, Page::Browse);
                }
                if !updates.is_empty() && button(ui, &format!("Обновить всё  {}", updates.len()), Some(Icon::Download), Kind::Secondary, 34.0).clicked() {
                    self.be.update_content(id.to_string(), updates.clone(), self.running.contains_key(id));
                }
                if icon_button(ui, Icon::Refresh, 34.0, "Обновить список").clicked() {
                    self.be.load_content(id.to_string());
                }
            });
        });
        ui.add_space(12.0);

        let q = self.mod_search.to_lowercase();
        let list: Vec<&ContentRow> = rows
            .iter()
            .filter(|m| (self.mod_filter == 0 || m.kind as usize == self.mod_filter - 1) && (q.is_empty() || m.name.to_lowercase().contains(&q)))
            .collect();

        let mut open_info: Option<ContentRow> = None;
        egui::Frame::new().fill(PANEL()).corner_radius(rr(12)).stroke(Stroke::new(1.0, BORDER())).inner_margin(6).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            let w = ui.available_width();
            let (hr, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
            let col_ver = hr.left() + w * 0.52;
            text_at(ui.painter(), pos2(hr.left() + 14.0, hr.center().y), Align2::LEFT_CENTER, "Название", semibold(12.0), TEXT_FAINT());
            text_at(ui.painter(), pos2(col_ver, hr.center().y), Align2::LEFT_CENTER, "Версия", semibold(12.0), TEXT_FAINT());
            text_at(ui.painter(), pos2(hr.right() - 70.0, hr.center().y), Align2::RIGHT_CENTER, "Вкл.", semibold(12.0), TEXT_FAINT());
            for m in &list {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 54.0), Sense::click());
                let t = hover_anim(ui, resp.id, resp.hovered());
                let p = ui.painter().clone();
                p.rect_filled(r, rr(8), RAISED().gamma_multiply(t));
                let ir = Rect::from_min_size(pos2(r.left() + 12.0, r.center().y - 17.0), Vec2::splat(34.0));
                match &m.icon_url {
                    Some(u) => {
                        p.rect_filled(ir, rr(8), RAISED());
                        egui::Image::from_uri(u.clone()).corner_radius(rr(8)).paint_at(ui, ir);
                    }
                    None => {
                        let (seed, hue) = seed_of(&m.name);
                        pixel_icon(&p, ir, seed, hue, 8.0);
                    }
                }
                let dim = if m.enabled { 1.0 } else { 0.45 };
                if !m.enabled {
                    p.rect_filled(ir, rr(8), PANEL().gamma_multiply(0.5));
                }
                let tw = col_ver - ir.right() - 24.0;
                text_wrap(&p, pos2(ir.right() + 12.0, r.center().y - 17.0), &m.name, semibold(14.0), TEXT().gamma_multiply(dim), tw, 1);
                if !m.author.is_empty() {
                    text_at(&p, pos2(ir.right() + 12.0, r.center().y + 10.0), Align2::LEFT_CENTER, format!("от {}", m.author), regular(12.5), TEXT_DIM().gamma_multiply(dim));
                }
                let vr = text_wrap(&p, pos2(col_ver, r.center().y - 8.0), &m.version, mono(12.5), TEXT_DIM().gamma_multiply(dim), w * 0.22, 1);
                if m.has_update {
                    let g = p.layout_no_wrap("↑ обновить".into(), medium(12.0), accent());
                    let br = Rect::from_min_size(pos2(vr.right() + 10.0, r.center().y - 11.0), vec2(g.size().x + 14.0, 22.0));
                    let bresp = ui.interact(br, resp.id.with("upd"), Sense::click());
                    let bt = hover_anim(ui, bresp.id, bresp.hovered());
                    p.rect_filled(br, rr(6), accent().gamma_multiply(0.14 + bt * 0.14));
                    p.galley(br.center() - g.size() / 2.0, g, accent());
                    if bresp.on_hover_text("Обновить до последней версии").clicked() {
                        self.be.update_content(id.to_string(), vec![m.file_path.clone()], self.running.contains_key(id));
                    }
                }
                let tr = Rect::from_center_size(pos2(r.right() - 89.0, r.center().y), vec2(38.0, 22.0));
                let mut en = m.enabled;
                if toggle_at(ui, tr, resp.id.with("tg"), &mut en).changed() {
                    self.be.toggle_content(id.to_string(), m.file_path.clone(), en);
                    if let Some(rows) = self.content.get_mut(id)
                        && let Some(row) = rows.iter_mut().find(|r| r.file_path == m.file_path)
                    {
                        row.enabled = en;
                    }
                }
                let mr = Rect::from_center_size(pos2(r.right() - 30.0, r.center().y), Vec2::splat(30.0));
                let more = icon_button_at(ui, mr, resp.id.with("more"), Icon::More, "");
                if resp.clicked() {
                    open_info = Some((*m).clone());
                }
                egui::Popup::menu(&more).show(|ui| {
                    if ui.button("Подробнее").clicked() {
                        open_info = Some((*m).clone());
                    }
                    if ui.button(egui::RichText::new("Удалить").color(lighten(DANGER(), 0.2))).clicked() {
                        self.be.remove_content(id.to_string(), m.file_path.clone());
                    }
                });
            }
            if list.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    if rows.is_empty() {
                        label(ui, "В сборке пока нет модов", semibold(15.0), TEXT_DIM());
                        label(ui, "Нажмите «Добавить», чтобы найти их на Modrinth", regular(13.0), TEXT_FAINT());
                    } else {
                        label(ui, "Ничего не найдено", semibold(15.0), TEXT_DIM());
                    }
                });
                ui.add_space(30.0);
            }
        });
        if let Some(m) = open_info {
            self.open_mod_info(id, &m);
        }
    }

    fn tab_screens(&mut self, ui: &mut Ui, id: &str) {
        let shots = self.screens.get(id).cloned();
        let Some(shots) = shots else {
            self.be.load_screens(id.to_string());
            self.screens.insert(id.to_string(), vec![]);
            return;
        };
        if shots.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| label(ui, "Скриншотов пока нет — в игре нажмите F2", semibold(15.0), TEXT_DIM()));
            return;
        }
        let gap = 10.0;
        let w = ui.available_width();
        let cols = ((w + gap) / (260.0 + gap)).floor().max(1.0) as usize;
        let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
        for row in shots.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for path in row {
                    let (r, resp) = ui.allocate_exact_size(vec2(cw, cw * 9.0 / 16.0), Sense::click());
                    let t = hover_anim(ui, resp.id, resp.hovered());
                    ui.painter().rect_filled(r, rr(10), RAISED());
                    egui::Image::from_uri(crate::sys::file_uri(path)).corner_radius(rr(10)).paint_at(ui, r.shrink(2.0 - t * 2.0));
                    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let cap = Rect::from_min_max(pos2(r.left(), r.bottom() - 26.0), r.max);
                    ui.painter().rect_filled(cap, egui::CornerRadius { nw: 0, ne: 0, sw: 10, se: 10 }, Color32::from_black_alpha((80.0 + 90.0 * t) as u8));
                    text_at(ui.painter(), pos2(cap.left() + 10.0, cap.center().y), Align2::LEFT_CENTER, name, medium(11.5), Color32::WHITE);
                    if resp.clicked() {
                        crate::sys::open(path);
                    }
                }
            });
            ui.add_space(gap - 8.0);
        }
    }

    fn tab_log(&mut self, ui: &mut Ui, id: &str) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            label(ui, "logs/latest.log", mono(12.5), TEXT_DIM());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if button(ui, "Копировать", Some(Icon::Copy), Kind::Secondary, 34.0).clicked() {
                    ui.ctx().copy_text(self.logs.get(id).cloned().unwrap_or_default());
                    self.toast(ui, "Журнал скопирован", Icon::Copy);
                }
                if button(ui, "Обновить", Some(Icon::Refresh), Kind::Secondary, 34.0).clicked() {
                    self.be.load_log(id.to_string());
                }
            });
        });
        ui.add_space(12.0);
        if self.is_running(id) {
            // живой журнал: перечитываем раз в 2 секунды
            let now = Self::now(ui);
            let k = Id::new("log-poll");
            let last: f64 = ui.ctx().data(|d| d.get_temp(k)).unwrap_or(0.0);
            if now - last > 2.0 {
                ui.ctx().data_mut(|d| d.insert_temp(k, now));
                self.be.load_log(id.to_string());
            }
        }
        let text = self.logs.get(id).cloned();
        egui::Frame::new().fill(INPUT()).corner_radius(rr(12)).stroke(Stroke::new(1.0, BORDER())).inner_margin(16).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            let w = ui.available_width();
            let Some(text) = text else {
                self.be.load_log(id.to_string());
                self.logs.insert(id.to_string(), String::new());
                return;
            };
            if text.is_empty() {
                label(ui, "Журнал пуст — сборка ещё не запускалась", regular(13.0), TEXT_DIM());
            }
            for line in text.lines() {
                let c = if line.contains("/ERROR]") || line.contains("Exception") {
                    lighten(DANGER(), 0.45)
                } else if line.contains("/WARN]") {
                    lighten(YELLOW(), 0.35)
                } else {
                    TEXT().gamma_multiply(0.82)
                };
                let mut job = LayoutJob::default();
                job.append(line, 0.0, TextFormat::simple(mono(12.0), c));
                job.wrap.max_width = w;
                ui.label(job);
            }
        });
    }
}
