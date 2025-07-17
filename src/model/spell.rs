use serde::{Deserialize};
use std::{string::ToString, fs::read_to_string};

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
	let file_read_result = read_to_string("res/spells.ron");
	if file_read_result.is_ok() {
		let spell_str = ron::from_str(file_read_result.unwrap().as_str());
		spell_str.unwrap()
	} else {
		panic!("Error: couldn't read spells file!");
	}
}
