use crate::model::Spell;

use ratatui::crossterm::event::{self, Event};
use std::time::Duration;

#[derive(Clone)]
pub enum GameEvent {
	PlaySpell(Spell)
}

#[derive(Clone)]
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
	match event::read() {
		Ok(event) => {
			match event {
				Event::Key(key_event) => {
					match key_event.kind {
						event::KeyEventKind::Press => {
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
						_ => { None }
					}
				},
				_ => None
			}
		},
		Err(_) => {
			panic!("Error reading event!");
		},
	}
}