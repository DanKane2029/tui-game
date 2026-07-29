use serde::Deserialize;
use std::fs::read_to_string;

#[derive(Debug, Deserialize, Clone)]
pub enum Element {
    None,
    Flame,
    Water,
    Shock,
    Earth,
    Gust,
    Ice,
    Toxic,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Status {
    None,
    Burned,
    Wet,
    Paralyzed,
    Poisoned,
    Sleep,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Spell {
    pub name: String,
    pub damage_value: u8,
    pub damage_element: Element,
    pub block_value: u8,
    pub block_element: Element,
}

#[allow(dead_code)]
pub fn get_spells() -> Vec<Spell> {
    match read_to_string("res/spells.ron") {
        Ok(contents) => ron::from_str(&contents).expect("Error: couldn't parse spells file!"),
        Err(_) => panic!("Error: couldn't read spells file!"),
    }
}
