use crate::app::*;
use crate::backend::Hit;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align, Align2, Color32, Id, Layout, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

const TYPES: [(&str, &str); 5] = [("Моды", "mod"), ("Сборки", "modpack"), ("Ресурспаки", "resourcepack"), ("Шейдеры", "shader"), ("Датапаки", "datapack")];
const SORTS: [(&str, &str); 5] = [
    ("По релевантности", "relevance"),
    ("По загрузкам", "downloads"),
    ("По подпискам", "follows"),
    ("Недавно обновлённые", "updated"),
    ("Новые", "newest"),
];

fn short_num(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1} млн", n as f64 / 1e6).replace('.', ",")
    } else if n >= 1_000 {
        format!("{:.1} тыс", n as f64 / 1e3).replace('.', ",")
    } else {
        n.to_string()
    }
}

impl App {
    fn run_search(&mut self, now: f64, more: bool) {
        let (_, pt) = TYPES[self.browse_tab];
        let target = self.browse_target.as_ref().and_then(|t| self.inst(t)).cloned();
        let (ver, loader) = match (&target, self.browse_match && pt != "modpack") {
            (Some(i), true) => (Some(i.game_version.clone()), (i.loader != "Vanilla").then(|| i.loader.to_lowercase())),
            _ => (None, None),
        };
        self.browse_offset = if more { self.hits.len() as u32 } else { 0 };
        self.browse_seq += 1;
        self.browse_loading = true;
        let cats: Vec<String> = self.browse_cats.iter().cloned().collect();
        if self.browse_source == 1 {
            self.be.search_cf(self.browse_seq, self.browse_query.trim().to_string(), pt, ver, loader, SORTS[self.browse_sort].1, self.browse_offset, cats, self.prefs.cf_key.clone());
        } else {
            self.be.search(self.browse_seq, self.browse_query.trim().to_string(), pt, ver, loader, SORTS[self.browse_sort].1, self.browse_offset, cats);
        }
        let _ = now;
    }

    pub fn page_browse(&mut self, ui: &mut Ui) {
        let now = Self::now(ui);
        ui.horizontal(|ui| {
            label(ui, if self.browse_source == 1 { "Обзор CurseForge" } else { "Обзор Modrinth" }, bold(26.0), TEXT());
            ui.add_space(12.0);
            let mut src = self.browse_source;
            if segmented(ui, Id::new("browse-src"), &["Modrinth", "CurseForge"], &mut src) {
                self.browse_source = src;
                self.browse_cats.clear();
                self.hits.clear();
                self.browse_sig.clear();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // куда ставить
                let cur = self.browse_target.as_ref().and_then(|t| self.inst(t)).map(|i| format!("{} · {} {}", i.name, i.loader, i.game_version)).unwrap_or_else(|| "Выберите сборку".into());
                let mut pick = None;
                egui::ComboBox::from_id_salt("target").selected_text(cur).width(260.0).show_ui(ui, |ui| {
                    for i in &self.instances {
                        if ui.selectable_label(self.browse_target.as_deref() == Some(&i.id), format!("{} · {} {}", i.name, i.loader, i.game_version)).clicked() {
                            pick = Some(i.id.clone());
                        }
                    }
                });
                if let Some(p) = pick {
                    self.installed.clear();
                    self.be.installed_ids(p.clone());
                    self.browse_target = Some(p);
                    self.browse_sig.clear();
                }
                label(ui, "Ставить в", regular(13.0), TEXT_DIM());
            });
        });
        ui.add_space(10.0);
        let labels: Vec<String> = TYPES.iter().map(|t| t.0.to_string()).collect();
        if tabs(ui, Id::new("browse-tabs"), &labels, &mut self.browse_tab) {
            self.browse_cats.clear();
        }
        let y = ui.cursor().top() - 8.0;
        ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, BORDER()));
        ui.add_space(14.0);

        if self.browse_source == 1 && self.prefs.cf_key.trim().is_empty() {
            self.cf_key_panel(ui);
            return;
        }
        let side_w = 230.0;
        let full_w = ui.available_width();
        ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(vec2(full_w - side_w - 20.0, 10.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(full_w - side_w - 20.0);
        ui.horizontal(|ui| {
            let w = ui.available_width() - 210.0 - 250.0;
            let resp = search_field(ui, &mut self.browse_query, if self.browse_source == 1 { "Искать на CurseForge" } else { "Искать на Modrinth" }, w);
            if resp.changed() {
                self.browse_edit_at = now;
            }
            egui::ComboBox::from_id_salt("browse-sort").selected_text(SORTS[self.browse_sort].0).width(196.0).show_ui(ui, |ui| {
                for (i, s) in SORTS.iter().enumerate() {
                    ui.selectable_value(&mut self.browse_sort, i, s.0);
                }
            });
            ui.add_space(8.0);
            toggle(ui, &mut self.browse_match);
            label(ui, "Только подходящие к сборке", regular(13.0), TEXT_DIM());
        });

        // поиск с задержкой после ввода
        let sig = format!("{}|{}|{}|{:?}|{}|{}|{:?}", self.browse_query.trim(), self.browse_tab, self.browse_sort, self.browse_target, self.browse_match, self.browse_source, self.browse_cats);
        if sig != self.browse_sig && now - self.browse_edit_at > 0.35 {
            self.browse_sig = sig;
            self.run_search(now, false);
        } else if sig != self.browse_sig {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
        }
        ui.add_space(6.0);

        let is_pack = TYPES[self.browse_tab].1 == "modpack";
        ui.horizontal(|ui| {
            let s = if self.browse_loading && self.hits.is_empty() { "Ищу…".to_string() } else { format!("Найдено {}", short_num(self.hits_total)) };
            label(ui, s, regular(13.0), TEXT_FAINT());
            if is_pack {
                label(ui, "· сборка ставится как новая сборка в библиотеке", regular(13.0), TEXT_FAINT());
            }
        });
        if let Some(e) = &self.browse_error {
            label(ui, format!("Ошибка поиска: {e}"), regular(13.0), lighten(DANGER(), 0.3));
        }
        ui.add_space(4.0);

        let hits = self.hits.clone();
        ui.spacing_mut().item_spacing.y = 10.0;
        for hit in &hits {
            self.hit_row(ui, hit, is_pack);
        }
        if (hits.len() as u64) < self.hits_total && !hits.is_empty() {
            ui.vertical_centered(|ui| {
                if button(ui, if self.browse_loading { "Загрузка…" } else { "Показать ещё" }, None, Kind::Secondary, 36.0).clicked() && !self.browse_loading {
                    self.run_search(now, true);
                }
            });
        }
        });
        ui.add_space(20.0);
        ui.allocate_ui_with_layout(vec2(side_w, 10.0), Layout::top_down(Align::Min), |ui| {
            ui.set_width(side_w);
            self.categories_panel(ui);
        });
        });
    }

    fn categories_panel(&mut self, ui: &mut Ui) {
        let (_, pt) = TYPES[self.browse_tab];
        let src = if self.browse_source == 1 { "cf" } else { "" };
        let key = format!("{src}|{pt}");
        let Some(cats) = self.categories.get(&key).cloned() else {
            self.categories.insert(key, vec![]);
            self.be.load_categories(src, pt, self.prefs.cf_key.clone());
            return;
        };
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                label(ui, "Фильтры", semibold(15.0), TEXT());
                if !self.browse_cats.is_empty() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Label::new(egui::RichText::new("Сбросить").font(medium(12.5)).color(accent())).sense(Sense::click())).clicked() {
                            self.browse_cats.clear();
                        }
                    });
                }
            });
            if cats.is_empty() {
                label(ui, "Загружаю категории…", regular(12.5), TEXT_FAINT());
                return;
            }
            let mut group = String::new();
            ui.spacing_mut().item_spacing.y = 2.0;
            for c in &cats {
                if c.group != group {
                    group = c.group.clone();
                    ui.add_space(8.0);
                    label(ui, group.to_uppercase(), semibold(11.0), TEXT_FAINT());
                    ui.add_space(2.0);
                }
                let mut on = self.browse_cats.contains(&c.id);
                if checkbox(ui, &mut on, &c.name).changed() {
                    if on {
                        self.browse_cats.insert(c.id.clone());
                    } else {
                        self.browse_cats.remove(&c.id);
                    }
                }
            }
        });
    }

    fn cf_key_panel(&mut self, ui: &mut Ui) {
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width().min(720.0));
            ui.spacing_mut().item_spacing.y = 8.0;
            label(ui, "Нужен ключ CurseForge API", semibold(17.0), TEXT());
            ui.add(egui::Label::new(egui::RichText::new("CurseForge, в отличие от Modrinth, пускает в каталог только с личным ключом. Он бесплатный: войдите на console.curseforge.com, создайте ключ в разделе «API keys» и вставьте его сюда — он хранится только на этом компьютере.").font(regular(13.5)).color(TEXT_DIM())).wrap());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.cf_key_edit).hint_text("$2a$10$…").password(true).desired_width(420.0).margin(egui::Margin::symmetric(10, 8)));
                if button(ui, "Сохранить", None, Kind::Primary, 34.0).clicked() && !self.cf_key_edit.trim().is_empty() {
                    self.prefs.cf_key = self.cf_key_edit.trim().to_string();
                    crate::prefs::save(&self.prefs);
                    self.categories.retain(|k, _| !k.starts_with("cf|"));
                    self.browse_sig.clear();
                }
                if button(ui, "Открыть консоль", Some(Icon::Globe), Kind::Secondary, 34.0).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab("https://console.curseforge.com/"));
                }
            });
        });
    }

    /// Установить проект из обзора (кнопка в строке и в окне проекта).
    pub fn install_hit(&mut self, h: &Hit, now: f64) {
        let id = h.project_id.clone();
        if h.project_type == "modpack" {
            if h.source == "cf" {
                self.toasts.push(Toast { text: "Сборки с CurseForge пока не ставятся — экспортируйте их как zip и импортируйте через Prism".into(), error: true, born: now });
                return;
            }
            self.installing.insert(id);
            self.be.install_modpack(h.clone());
        } else if let Some(t) = self.browse_target.clone() {
            self.installing.insert(id);
            if h.source == "cf" {
                let (gv, loader) = self.inst(&t).map(|i| (i.game_version.clone(), i.loader.clone())).unwrap_or_default();
                self.be.install_cf(t, h.clone(), gv, loader, self.prefs.cf_key.clone());
            } else {
                self.be.install_project(t, h.clone());
            }
        } else {
            self.toasts.push(Toast { text: "Сначала выберите сборку сверху справа".into(), error: false, born: now });
        }
    }

    fn hit_row(&mut self, ui: &mut Ui, h: &Hit, is_pack: bool) {
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(vec2(w, 116.0), Sense::click());
        let t = hover_anim(ui, resp.id, resp.hovered());
        let p = ui.painter().clone();
        p.rect(r, rr(12), PANEL().lerp_to_gamma(RAISED(), t), Stroke::new(1.0, BORDER().lerp_to_gamma(HOVER(), t)), StrokeKind::Inside);
        let ir = Rect::from_min_size(r.min + vec2(16.0, 18.0), Vec2::splat(80.0));
        p.rect_filled(ir, rr(14), RAISED());
        match &h.icon_url {
            Some(u) if !u.is_empty() => egui::Image::from_uri(u.clone()).corner_radius(rr(14)).paint_at(ui, ir),
            _ => {
                let (seed, hue) = seed_of(&h.project_id);
                pixel_icon(&p, ir, seed, hue, 14.0);
            }
        }
        let x = ir.right() + 16.0;
        let right_w = 170.0;
        let tw = r.right() - right_w - x;
        let nr = text_wrap(&p, pos2(x, r.top() + 16.0), &h.title, bold(17.0), TEXT(), tw * 0.7, 1);
        text_at(&p, pos2(nr.right() + 8.0, nr.center().y + 1.0), Align2::LEFT_CENTER, format!("от {}", h.author), regular(13.0), TEXT_DIM());
        text_wrap(&p, pos2(x, r.top() + 42.0), &h.description, regular(13.5), TEXT_DIM(), tw, 2);
        let mut tx = x;
        for tag in h.display_categories.iter().take(5) {
            let g = p.layout_no_wrap(tag.clone(), medium(11.5), TEXT_DIM());
            let cr = Rect::from_min_size(pos2(tx, r.bottom() - 30.0), vec2(g.size().x + 14.0, 20.0));
            if cr.right() > r.right() - right_w {
                break;
            }
            p.rect_filled(cr, rr(5), HOVER());
            p.galley(cr.center() - g.size() / 2.0, g, TEXT_DIM());
            tx = cr.right() + 6.0;
        }
        let sx = r.right() - right_w + 10.0;
        let updated = h.date_modified.get(..10).unwrap_or("").split('-').rev().collect::<Vec<_>>().join(".");
        for (k, (ic, s)) in [(Icon::Download, short_num(h.downloads)), (Icon::Heart, short_num(h.follows)), (Icon::Clock, updated)].iter().enumerate() {
            let yy = r.top() + 22.0 + k as f32 * 19.0;
            icon(&p, *ic, Rect::from_center_size(pos2(sx + 7.0, yy), Vec2::splat(13.0)), TEXT_FAINT());
            text_at(&p, pos2(sx + 20.0, yy), Align2::LEFT_CENTER, s, if k == 0 { semibold(13.0) } else { regular(12.5) }, if k == 0 { TEXT() } else { TEXT_DIM() });
        }
        let br = Rect::from_min_size(pos2(r.right() - right_w + 10.0, r.bottom() - 44.0), vec2(right_w - 26.0, 30.0));
        let bresp = ui.interact(br, resp.id.with("inst"), Sense::click());
        let bt = hover_anim(ui, bresp.id, bresp.hovered());
        let id = &h.project_id;
        if self.installing.contains(id) {
            let ph = (Self::now(ui) * 1.6).fract() as f32;
            p.rect_filled(br, rr(8), HOVER());
            let seg = Rect::from_min_size(pos2(br.left() + (br.width() - 50.0) * ph, br.top()), vec2(50.0, br.height()));
            p.rect_filled(seg, rr(8), accent().gamma_multiply(0.6));
            text_at(&p, br.center(), Align2::CENTER_CENTER, "Установка…", semibold(13.0), Color32::WHITE);
            ui.ctx().request_repaint();
        } else if self.installed.contains(id) && !is_pack {
            p.rect_filled(br, rr(8), GREEN().gamma_multiply(0.14));
            let g = p.layout_no_wrap("Установлен".into(), semibold(13.0), GREEN());
            let cx = br.center().x - (g.size().x + 20.0) / 2.0;
            icon(&p, Icon::Check, Rect::from_center_size(pos2(cx + 7.0, br.center().y), Vec2::splat(15.0)), GREEN());
            p.galley(pos2(cx + 20.0, br.center().y - g.size().y / 2.0), g, GREEN());
        } else {
            let can = is_pack || self.browse_target.is_some();
            p.rect_filled(br, rr(8), accent().lerp_to_gamma(lighten(accent(), 0.14), bt).gamma_multiply(if can { 1.0 } else { 0.4 }));
            let g = p.layout_no_wrap("Установить".into(), semibold(13.0), on_accent());
            let cx = br.center().x - (g.size().x + 22.0) / 2.0;
            icon(&p, Icon::Download, Rect::from_center_size(pos2(cx + 7.0, br.center().y), Vec2::splat(15.0)), on_accent());
            p.galley(pos2(cx + 22.0, br.center().y - g.size().y / 2.0), g, on_accent());
            if bresp.clicked() {
                self.install_hit(h, Self::now(ui));
            }
        }
        if resp.clicked() {
            self.open_project(h);
        }
    }
}
