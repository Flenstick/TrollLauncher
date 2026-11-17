use crate::app::*;
use crate::backend::Inst;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align, Align2, Id, Layout, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

const SORTS: [&str; 3] = ["Недавние", "По названию", "По времени в игре"];

impl App {
    pub fn page_library(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			label(ui, "Библиотека", bold(26.0), TEXT());
			ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
				ui.spacing_mut().item_spacing.x = 8.0;
				if button(ui, "Создать", Some(Icon::Plus), Kind::Primary, 34.0).clicked() {
					self.create.open = true;
				}
				if button(ui, "Импорт", Some(Icon::Folder), Kind::Secondary, 34.0).clicked() {
					self.import_open = true;
				}
				if button(ui, "Сборки Modrinth", Some(Icon::Browse), Kind::Secondary, 34.0).clicked() {
					self.browse_tab = 1;
					self.go(ui, Page::Browse);
				}
			});
		});
		ui.add_space(10.0);
		ui.horizontal(|ui| {
			egui::ComboBox::from_id_salt("lib-sort").selected_text(SORTS[self.lib_sort]).width(170.0).show_ui(ui, |ui| {
				for (i, s) in SORTS.iter().enumerate() {
					ui.selectable_value(&mut self.lib_sort, i, *s);
				}
			});
			search_field(ui, &mut self.lib_search, "Поиск сборок", 230.0);
		});
		ui.add_space(10.0);

        let loaders = ["", "Fabric", "Forge", "NeoForge", "Vanilla"];
        let labels: Vec<String> = ["Все", "Fabric", "Forge", "NeoForge", "Vanilla"]
            .iter()
            .enumerate()
            .map(|(k, n)| {
                let c = self.instances.iter().filter(|i| k == 0 || i.loader == loaders[k]).count();
                format!("{n}  {c}")
            })
            .collect();
        tabs(ui, Id::new("lib-tabs"), &labels, &mut self.lib_filter);
        let y = ui.cursor().top() - 8.0;
        ui.painter().hline(ui.max_rect().x_range(), y, Stroke::new(1.0, BORDER()));
        ui.add_space(10.0);

        let q = self.lib_search.to_lowercase();
        let mut list: Vec<Inst> = self
            .instances
            .iter()
            .filter(|i| (self.lib_filter == 0 || i.loader == loaders[self.lib_filter]) && (q.is_empty() || i.name.to_lowercase().contains(&q)))
            .cloned()
            .collect();
        match self.lib_sort {
            1 => list.sort_by_key(|i| i.name.to_lowercase()),
            2 => list.sort_by_key(|i| std::cmp::Reverse(i.played_secs)),
            _ => {}
        }

        if list.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                if self.instances.is_empty() {
                    label(ui, "Сборок пока нет", semibold(16.0), TEXT_DIM());
                    label(ui, "Создайте сборку, импортируйте её или установите с Modrinth", regular(13.5), TEXT_FAINT());
                } else {
                    label(ui, "Ничего не найдено", semibold(16.0), TEXT_DIM());
                }
            });
            return;
        }

        let gap = 14.0;
        let w = ui.available_width();
        let cols = ((w + gap) / (250.0 + gap)).floor().max(1.0) as usize;
        let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
        let (mut open, mut play) = (None, None);
        let now = Self::now(ui);
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for inst in row {
                    let state = if self.is_running(&inst.id) { 2 } else if self.is_launching(&inst.id, now) { 1 } else { 0 };
                    let (resp, pl) = instance_card(ui, inst, cw, state);
                    if pl {
                        play = Some(inst.id.clone());
                    } else if resp.clicked() {
                        open = Some(inst.id.clone());
                    }
                }
            });
            ui.add_space(gap - 8.0);
        }
        if let Some(i) = play {
            self.play(ui, &i);
        }
        if let Some(i) = open {
            self.go(ui, Page::Instance(i));
        }
    }
}

fn instance_card(ui: &mut Ui, inst: &Inst, w: f32, state: u8) -> (Response, bool) {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 146.0), Sense::click());
    let t = hover_anim(ui, resp.id, resp.hovered());
    let rect_d = rect.translate(vec2(0.0, -t * 2.0));
    let p = ui.painter().clone();
    p.rect(rect_d, rr(12), PANEL().lerp_to_gamma(RAISED(), t), Stroke::new(1.0, BORDER().lerp_to_gamma(HOVER(), t)), StrokeKind::Inside);
    let ir = Rect::from_min_size(rect_d.min + vec2(14.0, 14.0), Vec2::splat(58.0));
    App::inst_icon(ui, ir, inst, 11.0);
    let x = ir.right() + 12.0;
    text_wrap(&p, pos2(x, ir.top() + 2.0), &inst.name, semibold(15.0), TEXT(), rect_d.right() - x - 12.0, 2);
    text_at(&p, pos2(x, ir.bottom() - 8.0), Align2::LEFT_CENTER, format!("{} · {}", inst.game_version, inst.loader), regular(13.0), TEXT_DIM());

    let ly = rect_d.bottom() - 48.0;
    p.hline(rect_d.left() + 14.0..=rect_d.right() - 14.0, ly, Stroke::new(1.0, BORDER()));
    let by = rect_d.bottom() - 24.0;
    let running = state == 2;
    if running || state == 1 || inst.installing {
        let (s, c) = if running {
            ("Запущена", GREEN())
        } else if state == 1 {
            ("Запуск…", accent())
        } else {
            ("Устанавливается…", YELLOW())
        };
        p.circle_filled(pos2(rect_d.left() + 19.0, by), 4.0, c);
        text_at(&p, pos2(rect_d.left() + 30.0, by), Align2::LEFT_CENTER, s, semibold(13.0), c);
    } else {
        icon(&p, Icon::Clock, Rect::from_center_size(pos2(rect_d.left() + 20.0, by), Vec2::splat(13.0)), TEXT_FAINT());
        text_at(&p, pos2(rect_d.left() + 32.0, by), Align2::LEFT_CENTER, format!("{} · {}", fmt_played(inst.played_secs), fmt_ago(inst.last_played)), regular(12.5), TEXT_DIM());
    }

    let pr = Rect::from_center_size(pos2(rect_d.right() - 32.0, by), Vec2::splat(34.0));
    let presp = ui.interact(pr, resp.id.with("play"), Sense::click());
    let pt = hover_anim(ui, presp.id, presp.hovered());
    if !inst.installing {
        let bg = if running { DANGER() } else { accent() };
        let a = if running { 1.0 } else { 0.35 + 0.65 * t };
        p.circle_filled(pr.center(), 17.0, lighten(bg, pt * 0.14).gamma_multiply(a));
        let ic = if running { Icon::Stop } else { Icon::Play };
        icon(&p, ic, Rect::from_center_size(pr.center() + vec2(if running { 0.0 } else { 1.0 }, 0.0), Vec2::splat(15.0)), on_accent().gamma_multiply(0.6 + 0.4 * a));
    }
    let presp = presp.on_hover_text(if running { "Остановить" } else { "Играть" });
    (resp, presp.clicked() && !inst.installing)
}
