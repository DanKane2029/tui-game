use serde::{Deserialize};
use crate::model::spell::Element;
use std::{collections::HashMap, fs::read_to_string};

struct Behavior {
	
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Enemy {
	name: String,
	health: u8,
}

pub struct EnemyFactory {
	enemy_map: HashMap<String, Enemy>
}

impl EnemyFactory {
	pub fn new() -> Self {
		Self {
			enemy_map: Self::get_enemies()
		}
	}

	fn generate_enemy(&self, name: &str) -> Enemy {
		match self.enemy_map.get(name) {
			Some(enemy) => {
				enemy.clone()
			},
			None => {
				panic!("Enemy {:?} not found in enemy factory!", name);
			}
		}
	}

	fn get_enemies() -> HashMap<String, Enemy> {
		let file_read_result = read_to_string("res/enemies.ron");
		match file_read_result {
			Ok(file_content) => {
				match ron::from_str::<Vec<Enemy>>(&file_content) {
					Ok(enemies) => {
						enemies.into_iter().map(|e| (e.name.clone(), e.clone())).collect()
					},
					Err(_) => {
						panic!("Error: couldn't parse enemies file!");
					}
				}
			},
			Err(_) => {
				panic!("Error: couldn't read enemies file!");
			}
		}
	}
}