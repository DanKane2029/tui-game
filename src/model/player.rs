use crate::model::Spell;

#[derive(Clone)]
pub struct Player {
	health: u8,
	max_health: u8,
	magic: u8,
	max_magic: u8,
	spells: Vec<Spell>,
}

impl Player {
	pub fn new() -> Self {
		Self {
			health: 10,
			max_health: 10,
			magic: 5,
			max_magic: 5,
			spells: vec![],
		}
	}
}
