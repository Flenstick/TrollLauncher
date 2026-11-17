use crate::app::*;
use crate::backend::Inst;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{
	self, Align, Align2, Layout, Mesh, Painter, Rect, Response, Sense, Shape,
	Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};

impl App {
	pub fn page_home(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			ui.allocate_ui_with_layout(
				vec2((ui.available_width() - 230.0).max(300.0), 58.0),
				Layout::top_down(Align::Min),
				|ui| {
					heading(
						ui,
						"С возвращением.",
						Some("Твои сборки. Твои миры."),
					);
				},
			);
			ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
				let name = self
					.default_account()
					.map(|a| a.name.clone())
					.unwrap_or_else(|| "Войти в аккаунт".into());
				if button(ui, &name, Some(Icon::User), Kind::Secondary, 36.0)
					.clicked()
				{
					self.settings_section = ACCOUNTS_SECTION;
					self.go(ui, Page::Settings);
				}
			});
		});
		ui.add_space(24.0);

		let list: Vec<Inst> = home_instances(&self.instances)
			.into_iter()
			.cloned()
			.collect();
		self.home_hero(ui, list.first());
		ui.add_space(22.0);
		ui.horizontal(|ui| {
			label(ui, "Недавние сборки", semibold(18.0), TEXT());
			ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
				if button(ui, "Все сборки →", None, Kind::Secondary, 34.0)
					.clicked()
				{
					self.go(ui, Page::Library);
				}
			});
		});
		ui.add_space(10.0);
		let recent: Vec<Inst> = list.into_iter().skip(1).take(3).collect();
		if recent.is_empty() {
			label(
				ui,
				"Здесь появятся другие сборки из вашей библиотеки.",
				regular(13.5),
				TEXT_DIM(),
			);
		} else {
			let gap = 12.0;
			let cols = if ui.available_width() < 820.0 { 2 } else { 3 };
			let width =
				(ui.available_width() - gap * (cols - 1) as f32) / cols as f32;
			let (mut open, mut play) = (None, None);
			let now = Self::now(ui);
			for row in recent.chunks(cols) {
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = gap;
					for inst in row {
						let state = if self.is_running(&inst.id) {
							2
						} else if self.is_launching(&inst.id, now) {
							1
						} else {
							0
						};
						let (resp, start) = recent_card(ui, inst, width, state);
						if start {
							play = Some(inst.id.clone());
						} else if resp.clicked() {
							open = Some(inst.id.clone());
						}
					}
				});
				ui.add_space(gap);
			}
			if let Some(id) = play {
				self.play(ui, &id);
			}
			if let Some(id) = open {
				self.go(ui, Page::Instance(id));
			}
		}

		if !self.jobs.is_empty() || !self.installing.is_empty() {
			ui.add_space(20.0);
			for job in self.jobs.clone() {
				self.job_ui(ui, &job);
				ui.add_space(8.0);
			}
			if !self.installing.is_empty() {
				label(
					ui,
					format!("Устанавливаются моды: {}", self.installing.len()),
					regular(13.0),
					TEXT_DIM(),
				);
			}
		}
	}

	fn home_hero(&mut self, ui: &mut Ui, inst: Option<&Inst>) {
		let (rect, _) = ui.allocate_exact_size(
			vec2(ui.available_width(), 232.0),
			Sense::hover(),
		);
		hero_background(ui.painter(), rect);
		let left = rect.left() + 30.0;
		let width = (rect.width() - 60.0).min(700.0);
		text_at(
			ui.painter(),
			pos2(left, rect.top() + 30.0),
			Align2::LEFT_TOP,
			if inst.is_some() {
				"ПРОДОЛЖИТЬ ИГРУ"
			} else {
				"НАЧНИ СВОЁ ПРИКЛЮЧЕНИЕ"
			},
			medium(11.5),
			TEXT_DIM().lerp_to_gamma(BLUE(), 0.25),
		);
		text_wrap(
			ui.painter(),
			pos2(left, rect.top() + 60.0),
			inst.map(|i| i.name.as_str())
				.unwrap_or("Твоя первая сборка"),
			bold(30.0),
			TEXT(),
			width,
			1,
		);
		let detail = inst
			.map(|i| format!("Minecraft {} · {}", i.game_version, i.loader))
			.unwrap_or_else(|| {
				"Создай свою сборку или импортируй готовую.".into()
			});
		text_wrap(
			ui.painter(),
			pos2(left, rect.top() + 113.0),
			&detail,
			regular(15.0),
			TEXT_DIM(),
			width,
			1,
		);
		let area = Rect::from_min_size(
			pos2(left, rect.top() + 164.0),
			vec2(width, 42.0),
		);
		let mut buttons = ui.new_child(
			UiBuilder::new()
				.max_rect(area)
				.layout(Layout::left_to_right(Align::Center)),
		);
		let ui = &mut buttons;
		ui.spacing_mut().item_spacing.x = 10.0;
		if let Some(inst) = inst {
			let running = self.is_running(&inst.id);
			let launching = self.is_launching(&inst.id, Self::now(ui));
			let (title, symbol, kind) = if running {
				("Остановить", Icon::Stop, Kind::Danger)
			} else if inst.installing {
				("Устанавливается…", Icon::Download, Kind::Secondary)
			} else if launching {
				("Запуск…", Icon::Play, Kind::Secondary)
			} else {
				("Играть", Icon::Play, Kind::Primary)
			};
			ui.add_enabled_ui(
				running || (!launching && !inst.installing),
				|ui| {
					if button(ui, title, Some(symbol), kind, 42.0).clicked() {
						self.play(ui, &inst.id);
					}
				},
			);
			if button(ui, "Настроить сборку", None, Kind::Secondary, 42.0)
				.clicked()
			{
				self.go(ui, Page::Instance(inst.id.clone()));
			}
		} else {
			if button(
				ui,
				"Создать сборку",
				Some(Icon::Plus),
				Kind::Primary,
				42.0,
			)
			.clicked()
			{
				self.create.open = true;
			}
			if button(
				ui,
				"Импорт сборки",
				Some(Icon::Folder),
				Kind::Secondary,
				42.0,
			)
			.clicked()
			{
				self.import_open = true;
			}
		}
	}
}

fn home_instances(instances: &[Inst]) -> Vec<&Inst> {
	let mut ordered: Vec<_> = instances.iter().collect();
	ordered.sort_by_key(|i| {
		std::cmp::Reverse((
			i.last_played.is_some(),
			i.last_played.unwrap_or(i.created),
		))
	});
	ordered
}

fn rounded_outline(r: Rect, radius: f32) -> Vec<egui::Pos2> {
	let mut points = Vec::with_capacity(36);
	for (center, start) in [
		(pos2(r.right() - radius, r.top() + radius), -90.0_f32),
		(pos2(r.right() - radius, r.bottom() - radius), 0.0),
		(pos2(r.left() + radius, r.bottom() - radius), 90.0),
		(pos2(r.left() + radius, r.top() + radius), 180.0),
	] {
		for step in 0..=8 {
			let angle = (start + step as f32 * 90.0 / 8.0).to_radians();
			points.push(center + vec2(angle.cos(), angle.sin()) * radius);
		}
	}
	points
}

fn hero_background(p: &Painter, r: Rect) {
	let left = PANEL().lerp_to_gamma(accent(), 0.25);
	let right = PANEL();
	let color = |x: f32| {
		left.lerp_to_gamma(right, ((x - r.left()) / r.width()).clamp(0.0, 1.0))
	};
	let mut mesh = Mesh::default();
	mesh.colored_vertex(r.center(), color(r.center().x));
	for point in rounded_outline(r, 16.0 * round_k()) {
		mesh.colored_vertex(point, color(point.x));
	}
	let count = mesh.vertices.len() as u32 - 1;
	for i in 1..=count {
		mesh.add_triangle(0, i, if i == count { 1 } else { i + 1 });
	}
	p.add(Shape::mesh(mesh));
	let art_rect = Rect::from_min_max(
		pos2(r.right() - r.width() * 0.31, r.top() + 1.0),
		r.max - vec2(14.0, 1.0),
	);
	let art = p.with_clip_rect(art_rect.intersect(p.clip_rect()));
	for (offset, size, mix) in [
		(vec2(-18.0, -45.0), 140.0, 0.10),
		(vec2(60.0, 26.0), 160.0, 0.16),
		(vec2(-50.0, 120.0), 130.0, 0.08),
	] {
		let center = pos2(r.right() - 85.0, r.center().y) + offset;
		let points = [
			vec2(-1.0, -1.0),
			vec2(1.0, -1.0),
			vec2(1.0, 1.0),
			vec2(-1.0, 1.0),
		]
		.map(|v| {
			let angle = -25.0_f32.to_radians();
			center
				+ vec2(
					v.x * angle.cos() - v.y * angle.sin(),
					v.x * angle.sin() + v.y * angle.cos(),
				) * size * 0.5
		});
		art.add(Shape::convex_polygon(
			points.to_vec(),
			PANEL().lerp_to_gamma(accent(), mix),
			Stroke::NONE,
		));
	}
	p.rect_stroke(
		r, rr(16),
		Stroke::new(1.0, BORDER().lerp_to_gamma(accent(), 0.18)),
		StrokeKind::Inside,
	);
}

fn recent_card(
	ui: &mut Ui,
	inst: &Inst,
	w: f32,
	state: u8,
) -> (Response, bool) {
	let (rect, resp) = ui.allocate_exact_size(vec2(w, 84.0), Sense::click());
	let t = hover_anim(ui, resp.id, resp.hovered());
	let p = ui.painter().clone();
	p.rect(
		rect, rr(12),
		PANEL().lerp_to_gamma(RAISED(), t),
		Stroke::new(1.0, BORDER().lerp_to_gamma(HOVER(), t)),
		StrokeKind::Inside,
	);
	let ir = Rect::from_center_size(
		pos2(rect.left() + 38.0, rect.center().y),
		Vec2::splat(46.0),
	);
	App::inst_icon(ui, ir, inst, 10.0);
	let x = ir.right() + 13.0;
	text_wrap(
		&p,
		pos2(x, rect.top() + 18.0),
		&inst.name,
		semibold(14.5),
		TEXT(),
		rect.right() - x - 46.0,
		1,
	);
	text_wrap(
		&p,
		pos2(x, rect.top() + 45.0),
		format!("{} · {}", inst.game_version, inst.loader).as_str(),
		regular(12.5),
		TEXT_DIM(),
		rect.right() - x - 46.0,
		1,
	);
	let pr = Rect::from_center_size(
		pos2(rect.right() - 25.0, rect.center().y),
		Vec2::splat(30.0),
	);
	let enabled = !inst.installing && state != 1;
	let play = ui.interact(
		pr,
		resp.id.with("play"),
		if enabled {
			Sense::click()
		} else {
			Sense::hover()
		},
	);
	let (symbol, col) = if state == 2 {
		(Icon::Stop, DANGER())
	} else if inst.installing {
		(Icon::Download, TEXT_FAINT())
	} else {
		(Icon::Play, if enabled { BLUE() } else { TEXT_FAINT() })
	};
	icon(&p, symbol, pr.shrink(7.0), col);
	(
		resp,
		play.on_hover_text(if state == 2 {
			"Остановить"
		} else if inst.installing {
			"Устанавливается"
		} else if state == 1 {
			"Запуск…"
		} else {
			"Играть"
		})
		.clicked() && enabled,
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	fn instance(name: &str, created: i64, played: Option<i64>) -> Inst {
		Inst {
			id: name.into(),
			name: name.into(),
			game_version: "1.21.1".into(),
			loader: "Fabric".into(),
			icon: None,
			last_played: played
				.map(|t| chrono::DateTime::from_timestamp(t, 0).unwrap()),
			played_secs: 0,
			installing: false,
			created: chrono::DateTime::from_timestamp(created, 0).unwrap(),
		}
	}
	#[test]
	fn last_played_instance_stays_featured_after_an_import() {
		let input = vec![
			instance("new import", 500, None),
			instance("older game", 1, Some(200)),
			instance("last game", 1, Some(300)),
		];
		let order: Vec<_> = home_instances(&input)
			.iter()
			.map(|i| i.name.as_str())
			.collect();
		assert_eq!(order, ["last game", "older game", "new import"]);
	}
	#[test]
	fn new_library_uses_newest_instance_and_handles_empty_library() {
		assert!(home_instances(&[]).is_empty());
		let input =
			vec![instance("old", 100, None), instance("new", 200, None)];
		assert_eq!(home_instances(&input)[0].name, "new");
	}
}
