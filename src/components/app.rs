use crate::components::Component;
use crate::model::{InputEvent, get_input_event};

use ratatui::widgets::{Block, Borders, Paragraph};

pub struct App {
	title: String,
	text: String,
}

impl App {
	pub fn new() -> Self {
		Self {
			title: String::from("Title Text"),
			text: String::from("start "),
		}
	}
}

impl Component for App {
	fn init(&mut self) {
		
	}

	fn handle_events(&mut self) {
		
	}

	fn handle_key_events(&mut self) {
		match get_input_event() {
			Some(event) => {
				match event {
					InputEvent::Key(c) => {
						self.text.push(c);
					},
					_ => ()
				}
			},
			None => (),
		}
	}
	
	fn handle_mouse_events(&mut self) {
		todo!()
	}
	
	fn update(&mut self) {

	}
	
	fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
		f.render_widget(
			Paragraph::new(self.text.clone())
				.block(
					Block::default()
						.borders(Borders::all())
						.title(self.title.clone())
				),
			rect
		);
	}
}
