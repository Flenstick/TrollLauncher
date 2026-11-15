//! Перетаскивание сборок в окно и модов в выбранную сборку.
use crate::{
	app::{App, Page, Toast},
	backend::{Backend, Msg},
	theme::*,
	widgets::*,
};
use eframe::egui::{self, Align2, Id, LayerId, Margin, Order, Stroke};
use std::path::{Path, PathBuf};
use theseus::prelude::*;

#[derive(Debug, PartialEq, Eq)]
enum DropKind {
	Pack,
	Mod,
	Unsupported,
}

fn classify(path: &Path) -> DropKind {
	if path.is_dir() {
		return DropKind::Pack;
	}
	match path
		.extension()
		.and_then(|s| s.to_str())
		.unwrap_or("")
		.to_ascii_lowercase()
		.as_str()
	{
		"mrpack" | "zip" => DropKind::Pack,
		"jar" => DropKind::Mod,
		_ => DropKind::Unsupported,
	}
}

impl App {
	pub fn file_drop(&mut self, ctx: &egui::Context) {
		let (hovered, dropped) = ctx.input(|i| {
			(!i.raw.hovered_files.is_empty(), i.raw.dropped_files.clone())
		});
		if hovered {
			let painter = ctx.layer_painter(LayerId::new(
				Order::Tooltip,
				Id::new("file-drop-overlay"),
			));
			let rect = ctx.viewport_rect().shrink(12.0);
			painter.rect_filled(rect, 14, PANEL().gamma_multiply(0.96));
			painter.rect_stroke(
				rect,
				14,
				Stroke::new(2.0, accent()),
				egui::StrokeKind::Inside,
			);
			text_at(
				&painter,
				rect.center() - egui::vec2(0.0, 18.0),
				Align2::CENTER_CENTER,
				"Отпустите файлы",
				bold(24.0),
				TEXT(),
			);
			text_at(
				&painter,
				rect.center() + egui::vec2(0.0, 20.0),
				Align2::CENTER_CENTER,
				"MRPACK / ZIP — импорт сборки · JAR — добавить моды",
				regular(15.0),
				TEXT_DIM(),
			);
		}
		let mut unsupported = 0;
		for file in dropped {
			let path = file.path().to_path_buf();
			match classify(&path) {
				DropKind::Pack => {
					self.be.import_pack_path(path, self.prefs.cf_key.clone())
				}
				DropKind::Mod => {
					if self.dropped_mods.is_empty() {
						self.drop_target = match &self.page {
							Page::Instance(id) => Some(id.clone()),
							_ => None,
						};
					}
					if !self.dropped_mods.contains(&path) {
						self.dropped_mods.push(path);
					}
				}
				DropKind::Unsupported => unsupported += 1,
			}
		}
		if unsupported > 0 {
			self.toasts.push(Toast { text: format!("Не поддерживается файлов: {unsupported}. Перетащите .mrpack, .zip, .jar или папку сборки."), error: true, born: ctx.input(|i| i.time) });
		}
	}

	pub fn file_drop_dialog(&mut self, ctx: &egui::Context) {
		if self.dropped_mods.is_empty() {
			return;
		}
		let response = egui::Modal::new(Id::new("drop-mods"))
			.frame(
				egui::Frame::new()
					.fill(PANEL())
					.corner_radius(14)
					.stroke(Stroke::new(1.0, BORDER()))
					.inner_margin(Margin::same(22)),
			)
			.show(ctx, |ui| {
				ui.set_width(440.0);
				ui.spacing_mut().item_spacing.y = 10.0;
				label(ui, "Добавить моды", bold(20.0), TEXT());
				label(
					ui,
					format!("Файлов: {}", self.dropped_mods.len()),
					regular(13.5),
					TEXT_DIM(),
				);
				egui::ScrollArea::vertical()
					.max_height(140.0)
					.show(ui, |ui| {
						for file in &self.dropped_mods {
							label(
								ui,
								file.file_name()
									.unwrap_or_default()
									.to_string_lossy(),
								regular(13.0),
								TEXT_DIM(),
							);
						}
					});
				let selected = self
					.drop_target
					.as_ref()
					.and_then(|id| self.inst(id))
					.map(|i| i.name.clone())
					.unwrap_or_else(|| "Выберите сборку".into());
				egui::ComboBox::from_id_salt("drop-target")
					.selected_text(selected)
					.width(400.0)
					.show_ui(ui, |ui| {
						for instance in &self.instances {
							if !instance.installing {
								ui.selectable_value(
									&mut self.drop_target,
									Some(instance.id.clone()),
									&instance.name,
								);
							}
						}
					});
				if self.instances.is_empty() {
					label(
						ui,
						"Сначала создайте или импортируйте сборку.",
						regular(13.0),
						TEXT_DIM(),
					);
				}
				let valid = self
					.drop_target
					.as_ref()
					.and_then(|id| self.inst(id))
					.is_some_and(|i| !i.installing);
				ui.horizontal(|ui| {
					ui.add_enabled_ui(valid, |ui| {
						if button(
							ui,
							"Добавить",
							Some(Icon::Plus),
							Kind::Primary,
							34.0,
						)
						.clicked()
						{
							if let Some(id) = self.drop_target.take() {
								self.be.add_dropped_mods(
									id,
									std::mem::take(&mut self.dropped_mods),
								);
							}
						}
					});
					if button(ui, "Отмена", None, Kind::Secondary, 34.0)
						.clicked()
					{
						self.dropped_mods.clear();
					}
				});
			});
		if response.should_close() {
			self.dropped_mods.clear();
			self.drop_target = None;
		}
	}
}

impl Backend {
	pub fn add_dropped_mods(&self, id: String, paths: Vec<PathBuf>) {
		self.task(move |tx| async move {
			let mut added = 0;
			let mut errors = Vec::new();
			for path in paths {
				match instance::add_project_from_path(
					&id,
					&path,
					Some(ProjectType::Mod),
				)
				.await
				{
					Ok(_) => added += 1,
					Err(e) => errors.push(format!(
						"{}: {e}",
						path.file_name().unwrap_or_default().to_string_lossy()
					)),
				}
			}
			let text = if errors.is_empty() {
				format!("Добавлено модов: {added}")
			} else {
				format!(
					"Добавлено: {added}. Ошибок: {}. {}",
					errors.len(),
					errors[0]
				)
			};
			let _ = tx.send(Msg::Toast(text, !errors.is_empty()));
			let _ = tx.send(Msg::InstallDone(format!("local:{id}")));
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn dropped_files_are_classified_without_case_sensitivity() {
		for path in ["pack.mrpack", "pack.ZIP"] {
			assert_eq!(classify(Path::new(path)), DropKind::Pack);
		}
		for path in ["mod.jar", "mod.JAR"] {
			assert_eq!(classify(Path::new(path)), DropKind::Mod);
		}
		for path in ["image.png", "server.exe", "unknown"] {
			assert_eq!(classify(Path::new(path)), DropKind::Unsupported);
		}
		let folder = tempfile::tempdir().unwrap();
		assert_eq!(classify(folder.path()), DropKind::Pack);
	}
}
