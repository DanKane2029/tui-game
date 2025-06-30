use serde::{Deserialize};
use std::{string::ToString, fs::read_to_string};

#[derive(Debug, Deserialize)]
pub enum Element {
	None,
	Flame,
	Water,
	Shock,
	Earth,
	Gust,
	Ice,
}

#[derive(Debug, Deserialize)]
pub enum Status {
	None,
	Burned,
	Wet,
	Paralyzed,
	Poisoned,
	Sleep,

}

#[derive(Debug, Deserialize)]
pub struct Spell {
	pub name: String,
	pub damage_value: u8,
	pub damage_element: Element,
	pub block_value: u8,
	pub block_element: Element,
}

impl ToString for Spell {
	fn to_string(&self) -> String {
		self.name.clone()
	}
}

pub fn get_spells() -> Vec<Spell> {
	let file_read_result = read_to_string("res/spells.ron");
	if file_read_result.is_ok() {
		let spell_str = ron::from_str(file_read_result.unwrap().as_str());
		spell_str.unwrap()
	} else {
		panic!("Error: couldn't read spells file!");
	}
}
