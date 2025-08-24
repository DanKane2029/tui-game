use crate::model::{player::Player, Enemy};

pub enum TurnType {
	Player,
	Enemy
}

pub struct Fight {
	pub enemies: Vec<Enemy>,
	pub turn: TurnType,
	pub turn_count: u8
}

impl Fight {
	pub fn new(enemies: Vec<Enemy>) -> Self {
		Self {
			enemies,
			turn: TurnType::Player,
			turn_count: 0,
		}
	}
}

pub struct FightFactory {
	
}

pub struct FightManager {
	pub player: Player,
	pub fight: Fight,
}

impl FightManager {
	pub fn new(player: Player, fight: Fight) -> Self {
		Self {
			player,
			fight
		}
	}
}