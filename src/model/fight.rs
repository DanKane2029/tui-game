use crate::model::{player::Player, Enemy};

pub enum TurnType {
	Player,
	Enemy
}

pub struct Fight {
	enemies: Vec<Enemy>,
	turn: TurnType,
	turn_count: u8
}

pub struct FightFactory {
	
}

pub struct FightManager<'a> {
	player: &'a Player

}
