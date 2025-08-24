use crate::model::Player;
use crate::components::Component;

use ratatui::{text::Line, widgets::{List, ListDirection}};

pub struct PlayerStatsDisplay {
	player: &'static Player
}

impl Component for PlayerStatsDisplay {
	fn init(&mut self) {
		todo!()
	}

	async fn handle_events(&mut self) {
		todo!()
	}

	fn update(&mut self) {
		todo!()
	}

	fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
		f.render_widget(
			List::default()
				.direction(ListDirection::TopToBottom)
				.items([
					Line::from("TEST 1"),
					Line::from("TEST 2"),
					Line::from("TEST 3"),
				]),
			rect
		);
	}
}

