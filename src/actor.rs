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

use std::collections::HashSet;

use sdl2::pixels::Color;

use crate::dice;
use crate::display::{GREY};
use crate::items::{Item, Inventory};
use crate::map;
use crate::pathfinding::{find_path, manhattan_d};

use super::{GameState, Map, NPCTable};

#[derive(Debug)]
pub enum PirateType {
	Swab,
	Seadog,
}

#[derive(Debug)]
pub struct Player {
	pub name: String,
	pub ac: u8,
	pub max_stamina: u8,
	pub curr_stamina: u8,
	pub strength: u8,
	pub constitution: u8,
	pub dexterity: u8,
	pub verve: u8,
	pub prof_bonus: u8,
	pub row: usize,
	pub col: usize,
	pub inventory: Inventory,
	p_type: PirateType,
	pub on_ship: bool,
	pub bearing: u8,
	pub wheel: i8,
}

impl Player {
	pub fn mod_for_stat(stat: u8) -> i8 {
		(stat / 2) as i8 - 5
	}

	pub fn new_swab(name: String) -> Player {
		let stats = Player::roll_stats(2);
		let con_mod = Player::mod_for_stat(stats[3]);
		let hp = 8 + dice::roll(8, 4, con_mod);
		
		let mut p = Player { 
			name, ac: 10, 
			max_stamina: hp,
			curr_stamina: hp,
			dexterity: stats[0],
			verve: stats[1],
			strength: stats[2],
			constitution: stats[3],
			prof_bonus: 3,
			row:0, col:0, 
			inventory: Inventory::new(),
			p_type: PirateType::Swab,
			on_ship: false,
			bearing: 0,
			wheel: 0,
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
			max_stamina: hp,
			curr_stamina: hp,
			constitution: stats[0],
			strength: stats[1],
			dexterity: stats[2],
			verve: stats[3],
			prof_bonus: 4,
			row:0, col:0, 
			inventory: Inventory::new(),
			p_type: PirateType::Seadog,
			on_ship: false,
			bearing: 0,
			wheel: 0,
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

pub struct Monster {
	pub name: String,
	pub ac: u8,
	pub hp: u8,
	pub symbol: char,
	pub row: usize,
	pub col: usize,
	pub color: Color,
	pub hit_bonus: i8,
	pub dmg: u8,
	pub dmg_dice: u8,
	pub dmg_bonus: u8,
}

impl Monster {
	pub fn new(name: String, ac:u8, hp: u8, symbol: char, row: usize, col: usize, color: Color,
			hit_bonus: i8, dmg: u8, dmg_dice: u8, dmg_bonus: u8) -> Monster {
		Monster { name, ac, hp, symbol, row, col, color, hit_bonus, dmg, dmg_dice, dmg_bonus }
	}

	pub fn new_shark(row: usize, col: usize) -> Monster {
		let hp = dice::roll(8, 3, 0);
		Monster::new(String::from("shark"), 12, hp, '^', row, col, GREY,
			4, 8, 1, 2)
	}

	pub fn act(&mut self, state: &mut GameState, map: &Map, npcs: &mut NPCTable) -> Result<(), String> {
		shark_action(self, state, map, npcs)?;

		Ok(())
	}
}

fn shark_action(m: &mut Monster, state: &mut GameState, 
		map: &Map, npcs: &mut NPCTable) -> Result<(), String> {
	let d = manhattan_d(m.row, m.col, state.player.row, state.player.col);

	if d == 1 {
		if super::attack_player(state, m) {
			state.write_msg_buff("The shark bites you!");
			let dmg_roll = dice::roll(m.dmg, m.dmg_dice, m.dmg_bonus as i8);
			super::player_takes_dmg(&mut state.player, dmg_roll, "shark")?;
		} else {
			state.write_msg_buff("The shark misses!");
		}	
	} else if d < 50 {
		// Too far away and the sharks just ignore the player
		let mut water = HashSet::new();
		water.insert(map::Tile::Water);
		water.insert(map::Tile::DeepWater);
		let path = find_path(map, m.row, m.col, 
			state.player.row, state.player.col, &water);
		
		if path.len() > 1 {
			let new_loc = path[1];
			if npcs.contains_key(&new_loc) {
				let s = format!("The {} is blocked.", m.name);
				state.write_msg_buff(&s);
				return Ok(());
			} 

			m.row = new_loc.0;
			m.col = new_loc.1;
		}
	}

	Ok(())
}

