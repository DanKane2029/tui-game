use serde::{Deserialize};
use crate::game::spell::{Element, Status};
use std::fs::read_to_string;

#[derive(Debug, Deserialize)]
pub struct Enemy {
	name: String,
	health: u8,
	damage_value: u8,
	damage_element: Element,
}

pub fn get_enemies() -> Vec<Enemy> {
	let file_read_result = read_to_string("res/enemies.ron");
	if file_read_result.is_ok() {
		let enemies_str = ron::from_str(file_read_result.unwrap().as_str());
		enemies_str.unwrap()
	} else {
		panic!("Error: couldn't read enemies file!");
	}
}
