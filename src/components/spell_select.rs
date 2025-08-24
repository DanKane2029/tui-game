use std::error::Error;

use crate::components::Component;
use crate::model::{get_input_event, GameEvent, InputEvent, Spell, globals::PLAYER_SPELL_SLOTS};

use color_eyre::owo_colors::OwoColorize;
use ratatui::layout::Constraint;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::{layout::{Layout, Direction}, widgets::Paragraph};
use tokio::sync::broadcast::{Receiver, Sender};

pub struct SpellSelect {
	input_receiver: Receiver<InputEvent>,
	game_event_sender: Sender<GameEvent>,
	cur_select_index: usize,
	num_full_spell_slots: usize,
	spells: [Option<Spell>; PLAYER_SPELL_SLOTS],
	layout: Layout,
}

impl SpellSelect {
	pub fn new(spells: [Option<Spell>; 5], input_receiver: Receiver<InputEvent>, game_event_sender: Sender<GameEvent>) -> Self {
		
		let layout_constrains = (0..spells.len()).map(
					|_| Constraint::Percentage(100 / spells.len() as u16)
				).collect::<Vec<Constraint>>();
		let layout = Layout::default()
				.direction(Direction::Horizontal)
				.constraints(layout_constrains);
		Self {
			cur_select_index: 0,
			num_full_spell_slots: 0,
			spells,
			input_receiver,
			game_event_sender,
			layout,
		}
	}

	pub fn add_spell(&mut self, spell: Spell) -> Result<usize, String> {
		let mut result = Err("Spell slots full!".to_string());
		for (index, spell_slot) in self.spells.clone().iter().enumerate() {
			match spell_slot {
				Some(_) => { },
				None => {
					self.spells[index] = Some(spell.clone());
					self.num_full_spell_slots += 1;
					result = Ok(index);
					break;
				},
			}
		}
		result
	}
}

impl Component for SpellSelect {
	fn init(&mut self) {
		todo!()
	}

	async fn handle_events(&mut self) {
		match self.input_receiver.try_recv() {
			Ok(input_event) => {
				match input_event {
					InputEvent::LeftArrow => {
						if self.cur_select_index == 0 {
							self.cur_select_index = self.num_full_spell_slots - 1;
						} else {
							self.cur_select_index -= 1;
						}
					},
					InputEvent::RightArrow => {
						self.cur_select_index = (self.cur_select_index + 1) % self.num_full_spell_slots;
					},
					InputEvent::Enter => {
						let selected_spell = self.spells[self.cur_select_index].clone();
						match selected_spell {
							Some(spell) => {
								let _ = self.game_event_sender.send(GameEvent::PlaySpell(spell));
							},
							None => { },
						}
					}
					_ => { }
				}
			},
			Err(_) => { },
		}
	}

	fn update(&mut self) { }

	fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
		let select_layout = self.layout.split(rect);
		for (index, spell_result) in self.spells.iter().enumerate() {
			match spell_result {
				Some(spell) => {
					f.render_widget(
						Paragraph::new(
							format!("damage: {}\nblock: {}", spell.damage_value, spell.block_value)
						)
						.block(
							Block::default()
								.borders(Borders::all())
								.title(spell.name.clone())
								.style(
									if index == self.cur_select_index {
										Style::default().fg(Color::Green).bg(Color::Black)
									} else {
										Style::default().fg(Color::White).bg(Color::Black)
									}
								)
						),
						select_layout[index]
					);
				},
				None => {
					f.render_widget(
						Block::default()
							.borders(Borders::all())
							.title("none")
							.style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
						select_layout[index]
					);
				},
			}
			
		}
	}
}