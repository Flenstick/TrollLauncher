use crate::app::{App, Page};
use eframe::egui::{self, Event, Key, PointerButton, Ui};

impl App {
	pub(crate) fn navigate_history(&mut self, ui: &Ui, forward: bool) {
		if let Some(page) =
			history_step(&self.page, &mut self.back, &mut self.fwd, forward)
		{
			self.on_enter(&page);
			self.page = page;
			self.page_changed = Self::now(ui);
		}
	}

	pub(crate) fn mouse_history(&mut self, ui: &Ui) {
		if !self.ready
			|| ui.ctx().memory(|m| m.top_modal_layer().is_some())
			|| egui::Popup::is_any_open(ui.ctx())
		{
			return;
		}
		let action = ui.input(|i| history_action(&i.events));
		if let Some(forward) = action {
			self.navigate_history(ui, forward);
		}
	}
}

fn history_action(events: &[Event]) -> Option<bool> {
	let back = events.iter().any(|event| {
		matches!(
			event,
			Event::PointerButton {
				button: PointerButton::Extra1,
				pressed: false,
				..
			} | Event::Key {
				key: Key::BrowserBack,
				pressed: true,
				repeat: false,
				..
			}
		)
	});
	let forward = events.iter().any(|event| {
		matches!(
			event,
			Event::PointerButton {
				button: PointerButton::Extra2,
				pressed: false,
				..
			} | Event::Key {
				key: Key::BrowserForward,
				pressed: true,
				repeat: false,
				..
			}
		)
	});
	match (back, forward) {
		(true, false) => Some(false),
		(false, true) => Some(true),
		_ => None,
	}
}

fn history_step(
	page: &Page,
	back: &mut Vec<Page>,
	forward: &mut Vec<Page>,
	next: bool,
) -> Option<Page> {
	let (source, destination) = if next {
		(forward, back)
	} else {
		(back, forward)
	};
	let target = source.pop()?;
	destination.push(page.clone());
	Some(target)
}

#[cfg(test)]
mod tests {
	use super::*;
	fn mouse(button: PointerButton, pressed: bool) -> Event {
		Event::PointerButton {
			pos: egui::pos2(300.0, 200.0),
			button,
			pressed,
			modifiers: egui::Modifiers::NONE,
		}
	}
	#[test]
	fn side_buttons_navigate_once_on_release() {
		assert_eq!(history_action(&[mouse(PointerButton::Extra1, true)]), None);
		assert_eq!(
			history_action(&[mouse(PointerButton::Extra1, false)]),
			Some(false)
		);
		assert_eq!(
			history_action(&[mouse(PointerButton::Extra2, false)]),
			Some(true)
		);
		assert_eq!(
			history_action(&[mouse(PointerButton::Primary, false)]),
			None
		);
	}
	#[test]
	fn history_roundtrip_and_empty_history_keep_the_current_page() {
		let mut back = vec![Page::Home, Page::Library];
		let mut forward = vec![];
		let page =
			history_step(&Page::Settings, &mut back, &mut forward, false)
				.unwrap();
		assert!(page == Page::Library);
		let page = history_step(&page, &mut back, &mut forward, true).unwrap();
		assert!(page == Page::Settings);
		assert!(history_step(&page, &mut back, &mut forward, true).is_none());
		assert!(forward.is_empty());
		assert!(back == vec![Page::Home, Page::Library]);
	}
}
