use std::collections::HashMap;

use crate::model::{Player, Spell, Enemy};

pub struct GameManager {
	player: Player,
	spells: HashMap<String, Spell>,
	enemies: HashMap<String, Enemy>
}

impl GameManager {
	fn generate_fight() {

	}
}