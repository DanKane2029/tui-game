use crate::model::Spell;

pub struct Player {
	health: u8,
	max_health: u8,
	magic: u8,
	max_magic: u8,
	spells: Vec<Spell>,
}
