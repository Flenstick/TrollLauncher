//! Настройки интерфейса (тема, акцент, котики, масштаб) — ~/.local/share/TrollLauncher/native-ui.json.
//! При первом запуске котик берётся из настроек Prism Launcher.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Prefs {
	pub theme: String,
	/// Свой акцент (индекс в ACCENTS) или `None` — акцент темы
	pub accent: Option<usize>,
	pub custom_colors: crate::theme::CustomColors,
	pub scale: f32,
	pub animations: bool,
	/// id набора котиков или "none"
	pub cat: String,
	/// 0..=100
	pub cat_opacity: u8,
	/// индекс в cats::FITS
	pub cat_fit: usize,
	/// Ключ CurseForge API (console.curseforge.com) — для обзора CurseForge
	pub cf_key: String,
	pub sidebar_order: Vec<String>,
	/// Макет (look::PRESETS); после ручных правок остаётся последним выбранным
	pub preset: String,
	/// Где меню
	pub nav: crate::look::Nav,
	/// Скругление углов: 1 — обычно, 0 — квадратные
	pub round: f32,
	/// Плотность: 0.8 — тесно, 1.2 — просторно
	pub density: f32,
	/// Стиль иконок (look::ICON_STYLES) или `pack:<имя>`
	pub icons: String,
	/// Красить свои иконки в цвет темы
	pub icon_tint: bool,
	/// inter / pixel / mono / sys:<семейство> / file:<путь>
	pub font: String,
	/// Для sys: файлы обычного, medium, semibold, bold («путь#номер»)
	pub font_files: Vec<String>,
	/// Котик спрятан кнопкой на тулбаре PrismTroll
	pub cat_off: bool,
	/// PrismTroll: свёрнутые группы (id, "" — «Без группы») и выбранная сборка
	pub prism_collapsed: Vec<String>,
	pub prism_sel: Option<String>,
	/// Prism Launcher: иконки «pixel» (как в Prism) или «theme» (стиль иконок лаунчера)
	pub prism_icons: String,
	/// Prism Launcher: размер иконок сборок в сетке, px
	pub prism_tile: f32,
	pub prism_news: bool,
	pub prism_status: bool,
	pub prism_cat_button: bool,
	/// Скорость прокрутки колесом (множитель к 40 px за щелчок)
	pub scroll_speed: f32,
}

impl Default for Prefs {
	fn default() -> Self {
		Self {
			theme: "trollland".into(),
			accent: None,
			custom_colors: Default::default(),
			scale: 1.0,
			animations: true,
			cat: "kitteh".into(),
			cat_opacity: 100,
			cat_fit: 0,
			cf_key: String::new(),
			sidebar_order: Vec::new(),
			preset: "trollland".into(),
			nav: crate::look::Nav::Side,
			round: 1.0,
			density: 1.0,
			icons: "lines".into(),
			icon_tint: true,
			font: crate::look::FONT_INTER.into(),
			font_files: Vec::new(),
			cat_off: false,
			prism_collapsed: Vec::new(),
			prism_sel: None,
			prism_icons: "pixel".into(),
			prism_tile: 48.0,
			prism_news: true,
			prism_status: true,
			prism_cat_button: true,
			scroll_speed: 2.5,
		}
	}
}

pub fn xdg_data() -> PathBuf {
	crate::sys::data_root()
}
pub fn data_dir() -> PathBuf {
	xdg_data().join(crate::backend::APP_ID)
}
fn file() -> PathBuf {
	data_dir().join("native-ui.json")
}

pub fn load() -> Prefs {
	if let Some(p) = std::fs::read(file())
		.ok()
		.and_then(|b| serde_json::from_slice::<Prefs>(&b).ok())
	{
		let mut p = p;
		// 25.11.2026: точная копия Prism стала отдельным макетом «Prism Launcher» (id prismlauncher),
		// а id prism снова у «PrismTroll» (тулбар сверху)
		if p.preset == "prism" && p.nav == crate::look::Nav::Prism {
			p.preset = "prismlauncher".into();
		}
		return p;
	}
	let mut p = Prefs::default();
	// Котик из Prism: BackgroundCat, CatOpacity, CatFit, TheCat
	if let Ok(cfg) = std::fs::read_to_string(
		xdg_data().join("PrismLauncher/prismlauncher.cfg"),
	) {
		for line in cfg.lines() {
			let Some((k, v)) = line.split_once('=') else {
				continue;
			};
			match k.trim() {
				"BackgroundCat" if !v.trim().is_empty() => {
					p.cat = v.trim().to_string()
				}
				"CatOpacity" => p.cat_opacity = v.trim().parse().unwrap_or(100),
				"CatFit" => {
					p.cat_fit = match v.trim() {
						"fill" => 2,
						"strech" | "stretch" => 3,
						_ => 1,
					}
				}
				"TheCat" if v.trim() == "false" => {
					p.cat = crate::cats::NONE_ID.into()
				}
				_ => {}
			}
		}
	}
	p
}

pub fn save(p: &Prefs) {
	let _ = std::fs::create_dir_all(data_dir());
	if let Ok(b) = serde_json::to_vec_pretty(p) {
		let tmp = file().with_extension("json.tmp");
		if std::fs::write(&tmp, b).is_ok() {
			let _ = std::fs::rename(tmp, file());
		}
	}
}

impl Prefs {
	pub fn sidebar_ids(&self, live: &[String]) -> Vec<String> {
		let available: std::collections::HashSet<_> = live.iter().collect();
		let mut seen = std::collections::HashSet::new();
		self.sidebar_order
			.iter()
			.filter(|id| available.contains(id))
			.chain(live.iter())
			.filter(|id| seen.insert((*id).clone()))
			.cloned()
			.collect()
	}

	pub fn reorder_sidebar(
		&mut self,
		live: &[String],
		source: &str,
		target: &str,
		after: bool,
	) -> bool {
		if source == target {
			return false;
		}
		let mut ids = self.sidebar_ids(live);
		if !ids.iter().any(|id| id == target) {
			return false;
		}
		let Some(from) = ids.iter().position(|id| id == source) else {
			return false;
		};
		ids.remove(from);
		let to = ids.iter().position(|id| id == target).unwrap()
			+ usize::from(after);
		ids.insert(to, source.to_owned());
		if ids == self.sidebar_ids(live) {
			return false;
		}
		self.sidebar_order = ids;
		true
	}
}

#[cfg(test)]
mod sidebar_tests {
	use super::*;
	fn ids(values: &[&str]) -> Vec<String> {
		values.iter().map(|s| (*s).to_owned()).collect()
	}
	#[test]
	fn moving_in_both_directions_preserves_other_instances() {
		let mut p = Prefs::default();
		let live = ids(&["a", "b", "c", "d"]);
		assert!(p.reorder_sidebar(&live, "a", "c", true));
		assert_eq!(p.sidebar_order, ids(&["b", "c", "a", "d"]));
		assert!(p.reorder_sidebar(&live, "d", "b", false));
		assert_eq!(p.sidebar_order, ids(&["d", "b", "c", "a"]));
	}
	#[test]
	fn refresh_and_restart_keep_order_and_append_new_instances() {
		let mut p = Prefs::default();
		p.reorder_sidebar(&ids(&["a", "b", "c"]), "c", "a", false);
		let restored: Prefs =
			serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
		assert_eq!(
			restored.sidebar_ids(&ids(&["b", "new", "a", "c"])),
			ids(&["c", "a", "b", "new"])
		);
	}
	#[test]
	fn removed_instances_and_duplicate_preferences_are_ignored() {
		let mut p = Prefs::default();
		p.sidebar_order = ids(&["deleted", "b", "b", "a"]);
		assert_eq!(
			p.sidebar_ids(&ids(&["a", "b", "new"])),
			ids(&["b", "a", "new"])
		);
		assert!(!p.reorder_sidebar(&ids(&["a", "b"]), "missing", "b", true));
		assert!(!p.reorder_sidebar(&ids(&["a", "b"]), "a", "a", true));
	}
	#[test]
	fn existing_preferences_without_order_still_load() {
		let p: Prefs = serde_json::from_str(r#"{"theme":"midnight"}"#).unwrap();
		assert!(p.sidebar_order.is_empty());
		assert_eq!(p.theme, "midnight");
	}
}
