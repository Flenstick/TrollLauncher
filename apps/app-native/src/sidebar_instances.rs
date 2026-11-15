use crate::{
	app::{App, Page},
	theme::*,
	widgets::hover_anim,
};
use eframe::egui::{self, Id, Sense, Stroke, StrokeKind, Ui, Vec2, pos2};

struct SidebarInstance(String);

impl App {
	pub fn sidebar_instances(
		&mut self,
		ui: &mut Ui,
		bottom: f32,
	) -> Option<f32> {
		let live: Vec<_> =
			self.instances.iter().map(|i| i.id.clone()).collect();
		let ordered = self.prefs.sidebar_ids(&live);
		let instances: Vec<_> = ordered
			.iter()
			.filter_map(|id| self.inst(id).cloned())
			.collect();
		let mut active_y = None;
		let mut reorder = None;
		let height = (bottom - ui.cursor().top() - 180.0).max(44.0);
		egui::ScrollArea::vertical()
			.id_salt("sidebar-instances")
			.max_height(height)
			.auto_shrink([false, true])
			.show(ui, |ui| {
				ui.vertical_centered(|ui| {
					ui.spacing_mut().item_spacing.y = 6.0;
					for inst in instances {
						let (rect, _) = ui.allocate_exact_size(
							Vec2::splat(40.0),
							Sense::hover(),
						);
						let response = ui.interact(
							rect,
							Id::new(("sidebar-instance", &inst.id)),
							Sense::click_and_drag(),
						);
						response.dnd_set_drag_payload(SidebarInstance(
							inst.id.clone(),
						));
						let t = hover_anim(ui, response.id, response.hovered());
						let icon_rect = rect.shrink(4.0 - t);
						Self::inst_icon(ui, icon_rect, &inst, 9.0);
						if self.is_running(&inst.id) {
							let center = pos2(
								icon_rect.right() - 1.0,
								icon_rect.bottom() - 1.0,
							);
							ui.painter().circle_filled(center, 5.5, BG());
							ui.painter().circle_filled(center, 3.8, GREEN());
						}
						if self.page == Page::Instance(inst.id.clone())
							&& ui.is_rect_visible(rect)
						{
							active_y = Some(rect.center().y);
						}
						if response.dragged() {
							ui.ctx()
								.set_cursor_icon(egui::CursorIcon::Grabbing);
							ui.painter().rect_stroke(
								icon_rect.expand(2.0),
								10,
								Stroke::new(1.5, accent()),
								StrokeKind::Inside,
							);
						}
						let after = ui.input(|i| {
							i.pointer
								.interact_pos()
								.is_some_and(|p| p.y > rect.center().y)
						});
						if let Some(source) =
							response.dnd_hover_payload::<SidebarInstance>()
						{
							if source.0 != inst.id {
								let y = if after {
									rect.bottom() + 2.0
								} else {
									rect.top() - 2.0
								};
								ui.painter().hline(
									rect.x_range(),
									y,
									Stroke::new(2.0, accent()),
								);
							}
						}
						if let Some(source) =
							response.dnd_release_payload::<SidebarInstance>()
						{
							reorder = Some((
								source.0.clone(),
								inst.id.clone(),
								after,
							));
						}
						egui::Popup::context_menu(&response).show(|ui| {
							ui.label(&inst.name);
							ui.separator();
							let busy = inst.installing
								|| self.is_running(&inst.id)
								|| self.is_launching(&inst.id, Self::now(ui));
							if ui
								.add_enabled(
									!busy,
									egui::Button::new(
										egui::RichText::new("Удалить сборку")
											.color(lighten(DANGER(), 0.2)),
									),
								)
								.on_disabled_hover_text(
									"Дождитесь окончания установки или остановите сборку",
								)
								.clicked()
							{
								self.confirm_delete = Some(inst.id.clone());
								ui.close();
							}
						});
						if response
							.on_hover_text(format!(
								"{} · {}\nПеретащите для перестановки · ПКМ — меню",
								inst.name, inst.game_version
							))
							.clicked()
						{
							self.go(ui, Page::Instance(inst.id));
						}
					}
				});
			});
		if let Some((source, target, after)) = reorder {
			if self.prefs.reorder_sidebar(&live, &source, &target, after) {
				crate::prefs::save(&self.prefs);
			}
		}
		active_y
	}
}
