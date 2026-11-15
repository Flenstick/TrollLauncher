use crate::app::*;
use crate::backend::{Pick, SkinItem};
use crate::skin3d::{self, Pose};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align, Align2, Color32, Id, Layout, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

impl App {
    pub fn page_skins(&mut self, ui: &mut Ui) {
        let acc = self.default_account().cloned();
        let cap = match &acc {
            Some(a) => format!("Аккаунт {} · {}", a.name, if a.elyby { "Ely.by" } else { "Microsoft" }),
            None => "Войдите в аккаунт, чтобы надевать скины".into(),
        };
        heading(ui, "Скины", Some(&cap));
        ui.add_space(18.0);
        let now = Self::now(ui) as f32;
        let equipped_id = acc.as_ref().and_then(|a| self.equipped_skin_of(a));
        let sel: Option<SkinItem> = self.skin_sel.as_ref().and_then(|s| self.skins.iter().find(|i| &i.id == s)).cloned();

        ui.horizontal_top(|ui| {
            // ── большое 3D-превью
            ui.allocate_ui_with_layout(vec2(320.0, 10.0), Layout::top_down(Align::Min), |ui| {
                card_frame().show(ui, |ui| {
                    ui.set_width(284.0);
                    ui.spacing_mut().item_spacing.y = 10.0;
                    let (r, resp) = ui.allocate_exact_size(vec2(284.0, 380.0), Sense::click_and_drag());
                    ui.painter().rect_filled(r, 10, INPUT());
                    if resp.dragged() {
                        self.skin_yaw += resp.drag_delta().x * 0.012;
                    } else if !self.calm() {
                        self.skin_yaw += ui.input(|i| i.stable_dt).min(0.05) * 0.35;
                    }
                    if !self.calm() {
                        ui.ctx().request_repaint();
                    }
                    let shadow = egui::Shape::ellipse_filled(pos2(r.center().x, r.bottom() - 34.0), vec2(62.0, 10.0), Color32::from_black_alpha(110));
                    ui.painter().add(shadow);
                    match sel.as_ref().and_then(|s| self.skin_tex.get(&s.id).map(|t| (s, t))) {
                        Some((s, t)) => {
                            let pose = Pose { yaw: self.skin_yaw, pitch: -0.12, walk: if self.skin_walk { now * 4.5 } else { 0.0 }, idle: now * 1.4 };
                            skin3d::draw(&ui.painter().with_clip_rect(r), r.shrink(18.0), t.tex.id(), t.h, s.slim, pose);
                        }
                        None => {
                            text_at(ui.painter(), r.center(), Align2::CENTER_CENTER, "Нет скина", semibold(15.0), TEXT_FAINT());
                        }
                    }
                    let resp = resp.on_hover_text("Тяните мышью, чтобы покрутить");
                    if resp.double_clicked() {
                        self.skin_walk = !self.skin_walk;
                    }
                    if let Some(s) = &sel {
                        ui.horizontal(|ui| {
                            label(ui, &s.name, bold(19.0), TEXT());
                            if equipped_id.as_deref() == Some(&s.id) {
                                chip(ui, "Надет", GREEN().gamma_multiply(0.15), GREEN());
                            }
                        });
                        ui.horizontal(|ui| {
                            let mut v = usize::from(s.slim);
                            if segmented(ui, Id::new("skin-model"), &["Classic", "Slim"], &mut v) {
                                self.be.update_skin(s.id.clone(), None, None, Some(v == 1));
                            }
                            ui.add_space(6.0);
                            let mut w = self.skin_walk;
                            if toggle(ui, &mut w).changed() {
                                self.skin_walk = w;
                            }
                            label(ui, "Ходьба", regular(13.0), TEXT_DIM());
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            if equipped_id.as_deref() == Some(&s.id) {
                                button(ui, "Сейчас надет", Some(Icon::Check), Kind::Secondary, 36.0);
                            } else if let Some(a) = acc.clone() {
                                if button(ui, "Надеть", Some(Icon::Skins), Kind::Primary, 36.0).clicked() {
                                    self.be.equip_skin(a, s.clone());
                                }
                            } else if button(ui, "Войти", Some(Icon::User), Kind::Primary, 36.0).clicked() {
                                self.settings_section = ACCOUNTS_SECTION;
                                self.go(ui, Page::Settings);
                            }
                            let resp = button(ui, "", Some(Icon::Star), if s.favorite { Kind::Primary } else { Kind::Secondary }, 36.0);
                            if resp.on_hover_text(if s.favorite { "Убрать из избранного" } else { "В избранное" }).clicked() {
                                self.be.update_skin(s.id.clone(), None, Some(!s.favorite), None);
                            }
                            let more = button(ui, "", Some(Icon::More), Kind::Secondary, 36.0);
                            egui::Popup::menu(&more).show(|ui| {
                                if ui.button(egui::RichText::new("Удалить скин").color(lighten(DANGER(), 0.2))).clicked() {
                                    self.be.remove_skin_confirmed(s.id.clone(), s.name.clone());
                                }
                            });
                        });
                    }
                });
            });
            ui.add_space(22.0);

            // ── сетка
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let favs = self.skins.iter().filter(|s| s.favorite).count();
                    let labels = vec![format!("Все  {}", self.skins.len()), format!("Избранное  {favs}")];
                    tabs(ui, Id::new("skin-tabs"), &labels, &mut self.skin_tab);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if button(ui, "PNG-файл", Some(Icon::Plus), Kind::Primary, 34.0).clicked() {
                            self.be.pick(Pick::SkinFile);
                        }
                        if button(ui, "По нику", Some(Icon::User), Kind::Secondary, 34.0).clicked() {
                            self.nick.open = true;
                        }
                        if button(ui, "Из sfatm", Some(Icon::Download), Kind::Secondary, 34.0).on_hover_text("Забрать скины из гардероба мода sfatm в сборках").clicked() {
                            self.be.import_sfatm_skins();
                        }
                    });
                });
                ui.add_space(12.0);
                let list: Vec<SkinItem> = self.skins.iter().filter(|s| self.skin_tab == 0 || s.favorite).cloned().collect();
                if list.is_empty() {
                    ui.add_space(40.0);
                    label(ui, "Скинов пока нет", semibold(15.0), TEXT_DIM());
                    label(ui, "Добавьте PNG 64×64 или возьмите скин по нику игрока", regular(13.0), TEXT_FAINT());
                    return;
                }
                let gap = 12.0;
                let w = ui.available_width();
                let cols = ((w + gap) / (150.0 + gap)).floor().max(1.0) as usize;
                let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
                let mut pick = None;
                for row in list.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for s in row {
                            let (r, resp) = ui.allocate_exact_size(vec2(cw, 214.0), Sense::click());
                            let t = hover_anim(ui, resp.id, resp.hovered());
                            let sel = ui.ctx().animate_bool_with_time(Id::new("skinsel").with(&s.id), self.skin_sel.as_deref() == Some(&s.id), 0.15);
                            let p = ui.painter().clone();
                            p.rect(r, 12, PANEL().lerp_to_gamma(RAISED(), t), Stroke::new(1.0 + sel, BORDER().lerp_to_gamma(accent(), sel)), StrokeKind::Inside);
                            let fig = Rect::from_min_max(r.min + vec2(8.0, 10.0), pos2(r.right() - 8.0, r.bottom() - 38.0));
                            if let Some(tx) = self.skin_tex.get(&s.id) {
                                // при наведении модель поворачивается
                                let yaw = ease_to(ui.ctx(), resp.id.with("yaw"), if resp.hovered() { -0.55 } else { 0.42 }, 0.35);
                                let pose = Pose { yaw, pitch: -0.1, walk: 0.0, idle: 0.0 };
                                skin3d::draw(&p.with_clip_rect(r), fig, tx.tex.id(), tx.h, s.slim, pose);
                            }
                            text_wrap(&p, pos2(r.left() + 12.0, r.bottom() - 30.0), &s.name, semibold(13.5), TEXT(), cw - 40.0, 1);
                            if s.favorite {
                                icon(&p, Icon::Star, Rect::from_min_size(pos2(r.right() - 26.0, r.top() + 10.0), Vec2::splat(16.0)), accent());
                            }
                            if equipped_id.as_deref() == Some(&s.id) {
                                p.circle_filled(pos2(r.right() - 16.0, r.bottom() - 21.0), 4.0, GREEN());
                            }
                            if resp.clicked() {
                                pick = Some(s.id.clone());
                            }
                            egui::Popup::context_menu(&resp).show(|ui| {
                                ui.set_min_width(170.0);
                                if let Some(a) = acc.clone()
                                    && equipped_id.as_deref() != Some(&s.id)
                                    && ui.button("Надеть").clicked()
                                {
                                    self.be.equip_skin(a, s.clone());
                                }
                                if ui.button(if s.favorite { "Убрать из избранного" } else { "В избранное" }).clicked() {
                                    self.be.update_skin(s.id.clone(), None, Some(!s.favorite), None);
                                }
                                ui.separator();
                                if ui.button(egui::RichText::new("Удалить").color(lighten(DANGER(), 0.2))).clicked() {
                                    self.be.remove_skin_confirmed(s.id.clone(), s.name.clone());
                                }
                            });
                            if resp.double_clicked()
                                && let Some(a) = acc.clone()
                            {
                                self.be.equip_skin(a, s.clone());
                            }
                        }
                    });
                    ui.add_space(gap - 8.0);
                }
                if let Some(i) = pick {
                    self.skin_sel = Some(i);
                }
            });
        });
    }
}
