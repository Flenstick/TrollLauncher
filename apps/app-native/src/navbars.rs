//! Другие расположения меню (Настройки → Внешний вид → Меню): широкая панель слева с подписями,
//! тулбар сверху (как в Prism Launcher) и док снизу. Узкая панель — `App::sidebar` в app.rs.

use crate::app::*;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align, Align2, Id, Layout, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, ViewportCommand, pos2, vec2};

fn nav_items() -> [(Page, Icon, &'static str); 4] {
    [(Page::Home, Icon::Home, "Главная"), (Page::Library, Icon::Library, "Библиотека"), (Page::Browse, Icon::Browse, "Обзор"), (Page::Skins, Icon::Skins, "Скины")]
}

/// Строка широкой панели: иконка (или своя отрисовка слева) и подпись.
fn wide_row(ui: &mut Ui, ic: Option<Icon>, text: &str, active: bool) -> (Rect, egui::Response) {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), (36.0 * density()).max(30.0)), Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    let a = ui.ctx().animate_bool_with_time(resp.id.with("act"), active, 0.18);
    ui.painter().rect_filled(r, rr(8), HOVER().gamma_multiply((t * 0.6).max(a * 0.9)));
    if a > 0.0 {
        ui.painter().rect_filled(Rect::from_min_size(pos2(r.left(), r.center().y - 9.0), vec2(3.0, 18.0)), rr(2), accent().gamma_multiply(a));
    }
    let col = if active { TEXT() } else { TEXT_DIM().lerp_to_gamma(TEXT(), t) };
    if let Some(ic) = ic {
        icon(ui.painter(), ic, Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), Vec2::splat(19.0)), if active { accent().lerp_to_gamma(TEXT(), 0.15) } else { col });
    }
    ui.painter().with_clip_rect(r.shrink2(vec2(4.0, 0.0))).text(pos2(r.left() + 42.0, r.center().y), Align2::LEFT_CENTER, text, if active { semibold(14.0) } else { medium(14.0) }, col);
    (r, resp)
}

impl App {
    fn nav_active(&self, p: &Page) -> bool {
        self.page == *p || (*p == Page::Library && matches!(self.page, Page::Instance(_)))
    }

    fn account_text(&self) -> (String, String) {
        match self.default_account() {
            Some(a) => (a.name.clone(), if a.elyby { "Ely.by".into() } else { "Microsoft".into() }),
            None => ("Войти".into(), "в аккаунт".into()),
        }
    }

    // ───────────────────────── широкая панель ─────────────────────────

    pub fn sidebar_wide(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let head = Rect::from_min_size(full.min + vec2(18.0, 16.0), vec2(full.width() - 36.0, 30.0));
        logo(ui.painter(), Rect::from_min_size(head.min + vec2(0.0, 2.0), vec2(20.0, 26.0)));
        text_at(ui.painter(), pos2(head.left() + 30.0, head.center().y), Align2::LEFT_CENTER, "TrollLauncher", bold(16.0), TEXT());
        if ui.interact(head, Id::new("wide-logo"), Sense::click()).clicked() {
            self.go(ui, Page::Home);
        }
        ui.add_space(62.0);
        egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 0)).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (p, ic, name) in nav_items() {
                let active = self.nav_active(&p);
                if wide_row(ui, Some(ic), name, active).1.clicked() {
                    self.go(ui, p);
                }
            }
            ui.add_space(14.0);
            label(ui, "  СБОРКИ", semibold(11.0), TEXT_FAINT());
            ui.add_space(2.0);
            let live: Vec<_> = self.instances.iter().map(|i| i.id.clone()).collect();
            let insts: Vec<_> = self.prefs.sidebar_ids(&live).iter().filter_map(|id| self.inst(id).cloned()).collect();
            let height = (full.bottom() - ui.cursor().top() - 150.0).max(40.0);
            egui::ScrollArea::vertical().id_salt("wide-insts").max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for inst in insts {
                    let active = self.page == Page::Instance(inst.id.clone());
                    let (r, resp) = wide_row(ui, None, &inst.name, active);
                    Self::inst_icon(ui, Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), Vec2::splat(24.0)), &inst, 6.0);
                    if self.is_running(&inst.id) {
                        ui.painter().circle_filled(pos2(r.right() - 12.0, r.center().y), 3.8, GREEN());
                    }
                    if resp.clicked() {
                        self.go(ui, Page::Instance(inst.id.clone()));
                    }
                }
            });
            if wide_row(ui, Some(Icon::Plus), "Создать сборку", false).1.clicked() {
                self.create.open = true;
            }
        });
        let bottom = Rect::from_min_max(pos2(full.left() + 10.0, full.bottom() - 100.0), pos2(full.right() - 10.0, full.bottom() - 10.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(bottom).layout(Layout::bottom_up(Align::Min)), |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let (name, sub) = self.account_text();
            let (r, resp) = wide_row(ui, None, "", false);
            self.account_face(ui, Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), Vec2::splat(24.0)));
            text_at(ui.painter(), pos2(r.left() + 42.0, r.center().y - 7.0), Align2::LEFT_CENTER, name, semibold(13.5), TEXT());
            text_at(ui.painter(), pos2(r.left() + 42.0, r.center().y + 8.0), Align2::LEFT_CENTER, sub, regular(11.5), TEXT_DIM());
            if resp.clicked() {
                self.settings_section = ACCOUNTS_SECTION;
                self.go(ui, Page::Settings);
            }
            let active = self.page == Page::Settings;
            if wide_row(ui, Some(Icon::Settings), "Настройки", active).1.clicked() {
                self.go(ui, Page::Settings);
            }
        });
    }

    // ───────────────────────── тулбар сверху ─────────────────────────

    pub fn topnav(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let drag = ui.interact(full, Id::new("drag"), Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!max));
        }
        // на узком окне — без подписей
        let labels = full.width() > 1050.0;
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.add_space(14.0);
            let (lr, lresp) = ui.allocate_exact_size(vec2(20.0, 26.0), Sense::click());
            logo(ui.painter(), lr);
            if lresp.on_hover_text("TrollLauncher").clicked() {
                self.go(ui, Page::Home);
            }
            ui.add_space(10.0);
            ui.add_enabled_ui(!self.back.is_empty(), |ui| {
                if icon_button(ui, Icon::Back, 28.0, "Назад").clicked() {
                    self.navigate_history(ui, false);
                }
            });
            ui.add_enabled_ui(!self.fwd.is_empty(), |ui| {
                if icon_button(ui, Icon::Forward, 28.0, "Вперёд").clicked() {
                    self.navigate_history(ui, true);
                }
            });
            ui.add_space(8.0);
            ui.painter().vline(ui.cursor().left(), full.center().y - 11.0..=full.center().y + 11.0, Stroke::new(1.0, BORDER()));
            ui.add_space(8.0);
            for (p, ic, name) in nav_items() {
                let active = self.nav_active(&p);
                if top_button(ui, ic, name, active, labels).clicked() {
                    self.go(ui, p);
                }
            }
            if top_button(ui, Icon::Plus, "Создать", false, labels).clicked() {
                self.create.open = true;
            }
            let active = self.page == Page::Settings;
            if top_button(ui, Icon::Settings, "Настройки", active, labels).clicked() {
                self.go(ui, Page::Settings);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(6.0);
                self.window_buttons(ui);
                ui.add_space(8.0);
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
                let t = hover_anim(ui, resp.id, resp.hovered());
                ui.painter().rect_filled(r, rr(7), HOVER().gamma_multiply(t));
                self.account_face(ui, r.shrink(5.0));
                let (name, sub) = self.account_text();
                if resp.on_hover_text(format!("{name} · {sub}")).clicked() {
                    self.settings_section = ACCOUNTS_SECTION;
                    self.go(ui, Page::Settings);
                }
                ui.add_space(6.0);
                self.update_button(ui);
                self.downloads_button(ui);
                ui.add_space(6.0);
                self.running_pill(ui);
            });
        });
        ui.painter().hline(full.x_range(), full.bottom() - 0.5, Stroke::new(1.0, BORDER().gamma_multiply(0.6)));
    }

    /// Свернуть, развернуть, закрыть (справа налево).
    pub fn window_buttons(&mut self, ui: &mut Ui) {
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
    }

    // ───────────────────────── док снизу ─────────────────────────

    pub fn dock(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        enum Slot {
            Nav(Page, Icon, &'static str),
            Inst(String),
            Create,
            Sep,
            Settings,
            Account,
        }
        let mut slots: Vec<Slot> = nav_items().into_iter().map(|(p, i, n)| Slot::Nav(p, i, n)).collect();
        let live: Vec<_> = self.instances.iter().map(|i| i.id.clone()).collect();
        let ids = self.prefs.sidebar_ids(&live);
        let room = ((full.width() - 600.0) / 52.0).clamp(0.0, 8.0) as usize;
        if !ids.is_empty() && room > 0 {
            slots.push(Slot::Sep);
            slots.extend(ids.into_iter().take(room).map(Slot::Inst));
        }
        slots.push(Slot::Create);
        slots.push(Slot::Sep);
        slots.push(Slot::Settings);
        slots.push(Slot::Account);
        let (bw, sw, pad) = (48.0, 14.0, 10.0);
        let w = slots.iter().map(|s| if matches!(s, Slot::Sep) { sw } else { bw }).sum::<f32>() + pad * 2.0;
        let pill = Rect::from_center_size(pos2(full.center().x, full.center().y - 4.0), vec2(w, 60.0));
        ui.painter().rect(pill, rr(20), PANEL(), Stroke::new(1.0, BORDER()), StrokeKind::Inside);
        let mut x = pill.left() + pad;
        for (i, s) in slots.into_iter().enumerate() {
            if matches!(s, Slot::Sep) {
                ui.painter().vline(x + sw / 2.0, pill.center().y - 14.0..=pill.center().y + 14.0, Stroke::new(1.0, BORDER()));
                x += sw;
                continue;
            }
            let r = Rect::from_min_size(pos2(x, pill.top() + 6.0), vec2(bw, bw));
            x += bw;
            let resp = ui.interact(r, Id::new(("dock", i)), Sense::click());
            let t = hover_anim(ui, resp.id, resp.hovered());
            let grow = 1.0 + t * 0.14;
            let inner = Rect::from_center_size(r.center() - vec2(0.0, t * 3.0), Vec2::splat(40.0 * grow));
            let mut active = false;
            let tip: String;
            match &s {
                Slot::Nav(p, ic, name) => {
                    active = self.nav_active(p);
                    ui.painter().rect_filled(inner, rr(12), HOVER().gamma_multiply(t * 0.7));
                    let col = if active { accent().lerp_to_gamma(TEXT(), 0.15) } else { TEXT_DIM().lerp_to_gamma(TEXT(), t) };
                    icon(ui.painter(), *ic, Rect::from_center_size(inner.center(), Vec2::splat(22.0 * grow)), col);
                    tip = name.to_string();
                }
                Slot::Inst(id) => {
                    active = self.page == Page::Instance(id.clone());
                    if let Some(inst) = self.inst(id).cloned() {
                        Self::inst_icon(ui, inner.shrink(3.0), &inst, 10.0);
                        if self.is_running(id) {
                            ui.painter().circle_filled(inner.right_bottom() - vec2(4.0, 4.0), 5.0, PANEL());
                            ui.painter().circle_filled(inner.right_bottom() - vec2(4.0, 4.0), 3.5, GREEN());
                        }
                        tip = inst.name;
                    } else {
                        tip = String::new();
                    }
                }
                Slot::Create => {
                    ui.painter().rect_filled(inner, rr(12), HOVER().gamma_multiply(t * 0.7));
                    icon(ui.painter(), Icon::Plus, Rect::from_center_size(inner.center(), Vec2::splat(22.0 * grow)), TEXT_DIM().lerp_to_gamma(TEXT(), t));
                    tip = "Создать сборку".into();
                }
                Slot::Settings => {
                    active = self.page == Page::Settings;
                    ui.painter().rect_filled(inner, rr(12), HOVER().gamma_multiply(t * 0.7));
                    let col = if active { accent().lerp_to_gamma(TEXT(), 0.15) } else { TEXT_DIM().lerp_to_gamma(TEXT(), t) };
                    icon(ui.painter(), Icon::Settings, Rect::from_center_size(inner.center(), Vec2::splat(22.0 * grow)), col);
                    tip = "Настройки".into();
                }
                Slot::Account => {
                    self.account_face(ui, inner.shrink(6.0));
                    let (n, sub) = self.account_text();
                    tip = format!("{n} · {sub}");
                }
                Slot::Sep => unreachable!(),
            }
            if active {
                ui.painter().circle_filled(pos2(r.center().x, pill.bottom() - 5.0), 2.5, accent());
            }
            let resp = if tip.is_empty() { resp } else { resp.on_hover_text(tip) };
            if resp.clicked() {
                match s {
                    Slot::Nav(p, ..) => self.go(ui, p),
                    Slot::Inst(id) => self.go(ui, Page::Instance(id)),
                    Slot::Create => self.create.open = true,
                    Slot::Settings => self.go(ui, Page::Settings),
                    Slot::Account => {
                        self.settings_section = ACCOUNTS_SECTION;
                        self.go(ui, Page::Settings);
                    }
                    Slot::Sep => {}
                }
            }
        }
    }
}

/// Кнопка тулбара: иконка и подпись, активная — подчёркнута акцентом.
fn top_button(ui: &mut Ui, ic: Icon, text: &str, active: bool, labels: bool) -> egui::Response {
    let font = medium(13.5);
    let tw = if labels { ui.painter().layout_no_wrap(text.to_string(), font.clone(), TEXT()).size().x + 8.0 } else { 0.0 };
    let (r, resp) = ui.allocate_exact_size(vec2(36.0 + tw, 34.0), Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    let a = ui.ctx().animate_bool_with_time(resp.id.with("act"), active, 0.18);
    ui.painter().rect_filled(r, rr(7), HOVER().gamma_multiply((t * 0.7).max(a * 0.8)));
    if a > 0.0 {
        ui.painter().rect_filled(Rect::from_min_max(pos2(r.left() + 6.0, r.bottom() - 2.0), pos2(r.right() - 6.0, r.bottom())), rr(1), accent().gamma_multiply(a));
    }
    let col = if active { TEXT() } else { TEXT_DIM().lerp_to_gamma(TEXT(), t) };
    icon(ui.painter(), ic, Rect::from_center_size(pos2(r.left() + 18.0, r.center().y), Vec2::splat(17.0)), if active { accent().lerp_to_gamma(TEXT(), 0.15) } else { col });
    if labels {
        text_at(ui.painter(), pos2(r.left() + 33.0, r.center().y), Align2::LEFT_CENTER, text, font, col);
        resp
    } else {
        resp.on_hover_text(text)
    }
}
