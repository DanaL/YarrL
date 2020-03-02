// This file is part of YarrL, the pirate roguelike.
//
// YarrL is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// YarrL is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with YarrL.  If not, see <https://www.gnu.org/licenses/>.

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
