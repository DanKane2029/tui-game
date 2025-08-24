use crate::{components::player_stats_display::PlayerStatsDisplay, model::FightManager};
use crate::components::Component;

use std::collections::HashMap;
use ratatui::{layout::{Constraint, Direction, Layout}, widgets::{Block, Borders, Paragraph}};

pub struct FightDisplay {
	fight_manager: FightManager,
	layout_map: HashMap<&'static str, Layout>,
	player_stats_display: PlayerStatsDisplay,
}

impl<'a> FightDisplay {
	pub fn new(fight_manager: FightManager) -> Self {
		let num_enemies = fight_manager.fight.enemies.len();
		let layout_map = HashMap::from([
			(
				"fight_display",
				Layout::default()
					.direction(Direction::Horizontal)
					.constraints([Constraint::Percentage(20), Constraint::Fill(1)])
			),
			(
				"player",
				Layout::default()
					.direction(Direction::Vertical)
					.constraints([Constraint::Fill(1), Constraint::Percentage(20)])
			),
			(
				"enemies",
				Layout::default()
					.direction(Direction::Horizontal)
					.constraints((0..num_enemies).map(|_| Constraint::Percentage(100 / num_enemies as u16)))
			),
			(
				"enemy",
				Layout::default()
					.direction(Direction::Vertical)
					.constraints([Constraint::Percentage(20), Constraint::Fill(1)])
			)
		]);

		Self {
			fight_manager,
			layout_map,
			player_stats_display: PlayerStatsDisplay {
				player: &fight_manager.player
			}
		}
	}
}

impl Component for FightDisplay {
	fn init(&mut self) { }

	async fn handle_events(&mut self) {
		
	}

	fn update(&mut self) {
		
	}

	fn render(&mut self, f: &mut ratatui::Frame, rect: ratatui::prelude::Rect) {
		let fight_display_rects = self.layout_map.get("fight_display").unwrap().split(rect);
		let player_rects = self.layout_map.get("player").unwrap().split(fight_display_rects[0]);

		f.render_widget(
			Block::default()
				.title("Player Pic")
				.borders(Borders::all()),
			player_rects[0]
		);


		f.render_widget(
			Block::default()
				.title("Player Stats")
				.borders(Borders::all()),
			player_rects[1]
		);

		let enemies_rects = self.layout_map.get("enemies").unwrap().split(fight_display_rects[1]);

		for (index, enemy_rect) in enemies_rects.to_vec().iter().enumerate() {
			let enemy_rect = self.layout_map.get("enemy").unwrap().split(*enemy_rect);
			let enemy = self.fight_manager.fight.enemies[index].clone();
			f.render_widget(
				Block::default()
					.title(enemy.name)
					.borders(Borders::all()),
				enemy_rect[0]
			);

			f.render_widget(
				Block::default()
					.title(format!("Enemy Pic {}", index))
					.borders(Borders::all()),
				enemy_rect[1]
			);
		}
	}
}