use sdl2::pixels::Color;

use crate::items::Inventory;

pub struct Player {
	pub name: String,
	ac: u8,
	hp: u8,
	pub row: usize,
	pub col: usize,
	pub inventory: Inventory,
}

impl Player {
	pub fn new(name: String) -> Player {
		Player { name, ac: 10, hp: 10, row:0, col:0, inventory: Inventory::new() }
	}
}

pub trait Act {
	fn act(&mut self, state: &mut super::GameState);
	fn get_tile_info(&self) -> (Color, char);
}

pub struct Monster {
	ac: u8,
	hp: u8,
	symbol: char,
	row: usize,
	col: usize,
	color: Color,
}

impl Monster {
	pub fn new(ac:u8, hp: u8, symbol: char, row: usize, col: usize, color: Color) -> Monster {
		Monster { ac, hp, symbol, row, col, color }
	}
}

impl Act for Monster {
	fn act(&mut self, state: &mut super::GameState) {
		println!("My location is ({}, {})", self.row, self.col);
	}

	fn get_tile_info(&self) -> (Color, char) {
		(self.color, self.symbol)
	}
}
