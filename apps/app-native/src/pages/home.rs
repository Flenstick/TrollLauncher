use crate::app::*;
use crate::backend::Inst;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{Align, Align2, Layout, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

impl App {
    pub fn page_home(&mut self, ui: &mut Ui) {
        let caption = match self.instances.iter().find(|i| i.last_played.is_some()) {
            Some(i) => format!("Последняя игра — {}, {}", i.name, fmt_ago(i.last_played)),
            None => "Создайте сборку или поставьте готовую с Modrinth".into(),
        };
        heading(ui, "С возвращением", Some(&caption));
        ui.add_space(22.0);

        if self.instances.is_empty() {
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                label(ui, "Сборок пока нет", semibold(16.0), TEXT());
                label(ui, "Создайте свою или найдите готовую сборку в обзоре Modrinth.", regular(13.5), TEXT_DIM());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if button(ui, "Создать сборку", Some(Icon::Plus), Kind::Primary, 36.0).clicked() {
                        self.create.open = true;
                    }
                    if button(ui, "Сборки на Modrinth", Some(Icon::Browse), Kind::Secondary, 36.0).clicked() {
                        self.browse_tab = 1;
                        self.go(ui, Page::Browse);
                    }
                });
            });
        } else {
            section_title(ui, "Продолжить");
            ui.add_space(2.0);
            let gap = 12.0;
            let n = self.instances.len().min(4);
            let cw = (ui.available_width() - gap * 3.0) / 4.0;
            let (mut open, mut play) = (None, None);
            let list: Vec<Inst> = self.instances.iter().take(n).cloned().collect();
            let now = Self::now(ui);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for inst in &list {
                    let state = if self.is_running(&inst.id) { 2 } else if self.is_launching(&inst.id, now) { 1 } else { 0 };
                    let (resp, pl) = recent_card(ui, inst, cw, state);
                    if pl {
                        play = Some(inst.id.clone());
                    } else if resp.clicked() {
                        open = Some(inst.id.clone());
                    }
                }
            });
            if let Some(i) = play {
                self.play(ui, &i);
            }
            if let Some(i) = open {
                self.go(ui, Page::Instance(i));
            }
        }

        ui.add_space(26.0);
        self.trollland_card(ui);
        ui.add_space(26.0);

        ui.columns(2, |c| {
            // аккаунт
            let ui = &mut c[0];
            section_title(ui, "Аккаунт");
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                    self.account_face(ui, r);
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        match self.default_account().cloned() {
                            Some(a) => {
                                label(ui, &a.name, semibold(15.5), TEXT());
                                label(ui, if a.elyby { "Ely.by" } else { "Лицензия Microsoft" }, regular(13.0), if a.elyby { GREEN() } else { BLUE() });
                            }
                            None => {
                                label(ui, "Вход не выполнен", semibold(15.5), TEXT());
                                label(ui, "Войдите через Microsoft или Ely.by", regular(13.0), TEXT_DIM());
                            }
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "Аккаунты", Some(Icon::User), Kind::Secondary, 34.0).clicked() {
                            self.settings_section = 3;
                            self.go(ui, Page::Settings);
                        }
                    });
                });
            });
            // загрузки
            let ui = &mut c[1];
            section_title(ui, "Загрузки");
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                if self.jobs.is_empty() && self.installing.is_empty() {
                    label(ui, "Сейчас ничего не скачивается", regular(13.5), TEXT_DIM());
                }
                for j in &self.jobs {
                    label(ui, &j.title, medium(13.5), TEXT());
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
                    progress_bar(ui.painter(), r, j.progress.unwrap_or(0.05));
                }
                if !self.installing.is_empty() {
                    label(ui, format!("Ставятся моды: {}", self.installing.len()), medium(13.5), TEXT());
                }
            });
        });
    }

    fn trollland_card(&mut self, ui: &mut Ui) {
        let tl = self.instances.iter().find(|i| i.name.to_lowercase().contains("trollland")).cloned();
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(w, 140.0), Sense::hover());
        let p = ui.painter().clone();
        p.rect(rect, 14, PANEL(), Stroke::new(1.0, BORDER()), StrokeKind::Inside);
        let bw = if rect.width() < 900.0 { rect.width() * 0.26 } else { (rect.width() * 0.34).min(380.0) };
        let banner = Rect::from_min_max(pos2(rect.right() - bw, rect.top()), rect.max).shrink(10.0);
        landscape(&p, banner, 4242, 10.0);
        let lr = Rect::from_min_size(pos2(rect.left() + 26.0, rect.center().y - 36.0), vec2(56.0, 72.0));
        logo(&p, lr);
        let x = lr.right() + 24.0;
        text_at(&p, pos2(x, rect.top() + 36.0), Align2::LEFT_CENTER, "TrollLand", bold(22.0), TEXT());
        text_at(&p, pos2(x, rect.top() + 62.0), Align2::LEFT_CENTER, "Сервер Stamonian Federation", regular(13.5), TEXT_DIM());
        let sub = match &tl {
            Some(i) => format!("{} · {} {} · {} в игре", i.name, i.loader, i.game_version, fmt_played(i.played_secs)),
            None => "Сборка TrollLand не найдена в библиотеке".into(),
        };
        text_at(&p, pos2(x, rect.top() + 86.0), Align2::LEFT_CENTER, sub, regular(13.0), TEXT_FAINT());

        let area = Rect::from_min_max(pos2(banner.left() - 330.0, rect.bottom() - 58.0), pos2(banner.left() - 18.0, rect.bottom() - 16.0));
        let mut play = false;
        let now = Self::now(ui);
        ui.scope_builder(UiBuilder::new().max_rect(area).layout(Layout::right_to_left(Align::Center)), |ui| {
            match &tl {
                Some(i) => {
                    let resp = if self.is_running(&i.id) {
                        button(ui, "Остановить", Some(Icon::Stop), Kind::Danger, 42.0)
                    } else if self.is_launching(&i.id, now) {
                        button(ui, "Запуск…", Some(Icon::Play), Kind::Secondary, 42.0)
                    } else {
                        button(ui, "Играть на TrollLand", Some(Icon::Play), Kind::Primary, 42.0)
                    };
                    play = resp.clicked();
                }
                None => {
                    if button(ui, "Создать сборку", Some(Icon::Plus), Kind::Secondary, 42.0).clicked() {
                        self.create.open = true;
                        self.create.name = "TrollLand".into();
                    }
                }
            }
        });
        if play && let Some(i) = tl {
            self.play(ui, &i.id);
        }
    }
}

fn recent_card(ui: &mut Ui, inst: &Inst, w: f32, state: u8) -> (Response, bool) {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 78.0), Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    let p = ui.painter().clone();
    p.rect(rect, 12, PANEL().lerp_to_gamma(RAISED(), t), Stroke::new(1.0, BORDER().lerp_to_gamma(HOVER(), t)), StrokeKind::Inside);
    let ir = Rect::from_min_size(pos2(rect.left() + 14.0, rect.center().y - 25.0), Vec2::splat(50.0));
    App::inst_icon(ui, ir, inst, 10.0);
    let x = ir.right() + 12.0;
    let tw = rect.right() - x - 52.0;
    text_wrap(&p, pos2(x, rect.top() + 15.0), &inst.name, semibold(14.5), TEXT(), tw, 1);
    text_at(&p, pos2(x, rect.top() + 44.0), Align2::LEFT_CENTER, format!("{} · {}", inst.game_version, inst.loader), regular(12.5), TEXT_DIM());
    let (sub, col) = match state {
        2 => ("Запущена".to_string(), GREEN()),
        1 => ("Запуск…".to_string(), accent()),
        _ if inst.installing => ("Устанавливается…".to_string(), YELLOW()),
        _ => (fmt_ago(inst.last_played), TEXT_FAINT()),
    };
    text_at(&p, pos2(x, rect.top() + 61.0), Align2::LEFT_CENTER, sub, regular(12.0), col);

    let pr = Rect::from_center_size(pos2(rect.right() - 30.0, rect.center().y), Vec2::splat(34.0));
    let presp = ui.interact(pr, resp.id.with("play"), Sense::click());
    let pt = hover_anim(ui, presp.id, presp.hovered());
    let running = state == 2;
    let vis = if running { 1.0 } else { t };
    if vis > 0.01 && !inst.installing {
        let bg = if running { DANGER() } else { accent() };
        p.circle_filled(pr.center(), 17.0 + pt * 1.5, lighten(bg, pt * 0.12).gamma_multiply(vis));
        let ic = if running { Icon::Stop } else { Icon::Play };
        icon(&p, ic, Rect::from_center_size(pr.center() + vec2(if running { 0.0 } else { 1.0 }, 0.0), Vec2::splat(16.0)), on_accent().gamma_multiply(vis));
    }
    let clicked = presp.clicked() && vis > 0.5 && !inst.installing;
    (resp, clicked)
}
