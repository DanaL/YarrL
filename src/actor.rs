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

extern crate rand;

use sdl2::pixels::Color;

use crate::dice;
use crate::items::{Item, Inventory};

#[derive(Debug)]
pub enum PirateType {
	Swab,
	Seadog,
}

#[derive(Debug)]
pub struct Player {
	pub name: String,
	pub ac: u8,
	pub stamina: u8,
	pub strength: u8,
	pub constitution: u8,
	pub dexterity: u8,
	pub verve: u8,
	pub prof_bonus: u8,
	pub row: usize,
	pub col: usize,
	pub inventory: Inventory,
	p_type: PirateType,
}

impl Player {
	fn mod_for_stat(stat: u8) -> i8 {
		(stat / 2) as i8 - 5
	}

	pub fn new_swab(name: String) -> Player {
		let stats = Player::roll_stats(2);
		let con_mod = Player::mod_for_stat(stats[3]);
		let hp = 8 + dice::roll(8, 4, con_mod);
		
		let mut p = Player { 
			name, ac: 10, 
			stamina: hp,
			dexterity: stats[0],
			verve: stats[1],
			strength: stats[2],
			constitution: stats[3],
			prof_bonus: 3,
			row:0, col:0, 
			inventory: Inventory::new(),
			p_type: PirateType::Swab,
		};

		p.inventory.add(Item::get_item("rusty cutlass").unwrap());
		p.inventory.add(Item::get_item("leather jerkin").unwrap());
		p.inventory.add(Item::get_item("draught of rum").unwrap());
		p.inventory.add(Item::get_item("draught of rum").unwrap());
		p.inventory.add(Item::get_item("draught of rum").unwrap());

		p.inventory.toggle_slot('a');
		p.inventory.toggle_slot('b');

		p.calc_ac();

		p
	}

	pub fn new_seadog(name: String) -> Player {
		let stats = Player::roll_stats(0);
		let con_mod = Player::mod_for_stat(stats[0]);
		let hp = 8 + dice::roll(8, 6, con_mod);
		
		let mut p = Player { 
			name, ac: 10, 
			stamina: hp,
			constitution: stats[0],
			strength: stats[1],
			dexterity: stats[2],
			verve: stats[3],
			prof_bonus: 4,
			row:0, col:0, 
			inventory: Inventory::new(),
			p_type: PirateType::Seadog,
		};

		p.inventory.add(Item::get_item("rusty cutlass").unwrap());
		p.inventory.add(Item::get_item("flintlock pistol").unwrap());
		p.inventory.add(Item::get_item("overcoat").unwrap());
		p.inventory.add(Item::get_item("battered tricorn").unwrap());
		p.inventory.add(Item::get_item("draught of rum").unwrap());
		p.inventory.add(Item::get_item("draught of rum").unwrap());
		p.inventory.add(Item::get_item("draught of rum").unwrap());
		for _ in 0..12 {
			p.inventory.add(Item::get_item("lead ball").unwrap());
		}

		p.inventory.toggle_slot('a');
		p.inventory.toggle_slot('b');
		p.inventory.toggle_slot('c');
		p.inventory.toggle_slot('d');

		p.calc_ac();

		p
	}

	pub fn calc_ac(&mut self) {
		let mut total: i8 = 10;
		total += self.inventory.total_armour_value();
		total += Player::mod_for_stat(self.dexterity);

		self.ac = if total < 0 {
			0
		} else {
			total as u8
		};
	}

	fn roll_stats(bonus: i8) -> Vec<u8> {
		let mut v = Vec::new();
	
		for _ in 0..4 {
			v.push(dice::roll(6, 3, bonus));
		}
		v.sort();
		v.reverse();

		v
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
