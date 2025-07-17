use ratatui::crossterm::event::{self, Event};
use std::time::Duration;

pub enum GameEvent {

}

pub enum InputEvent {
	Key(char),
	UpArrow,
	DownArrow,
	RightArrow,
	LeftArrow,
	Enter,
	Esc,
}

pub fn get_input_event() -> Option<InputEvent> {
	match event::poll(Duration::from_millis(1)) {
		Ok(is_event) => {
			match is_event {
				true => match event::read() {
					Ok(event) => {
						match event {
							Event::Key(key_event) => {
								match key_event.code {
									event::KeyCode::Enter => Some(InputEvent::Enter),
									event::KeyCode::Left => Some(InputEvent::LeftArrow),
									event::KeyCode::Right => Some(InputEvent::RightArrow),
									event::KeyCode::Up => Some(InputEvent::UpArrow),
									event::KeyCode::Down => Some(InputEvent::DownArrow),
									event::KeyCode::Char(c) => Some(InputEvent::Key(c)),
									event::KeyCode::Esc => Some(InputEvent::Esc),
									_ => { None }
								}
							},
							_ => None
						}
					},
					Err(_) => {
						panic!("Error reading event!");
					},
				},
				false => { None },
			}
		},
		Err(_) => {
			panic!("Error polling event");
		},
	}
}