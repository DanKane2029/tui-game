use std::collections::HashMap;

use crate::model::{Enemy, Player, Spell};

pub struct GameManager {
    player: Player,
    spells: HashMap<String, Spell>,
    enemies: HashMap<String, Enemy>,
}

impl GameManager {
    fn generate_fight() {}
}
