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

use rand::thread_rng;
use rand::Rng;
use rand::seq::SliceRandom;

use std::collections::{HashMap, HashSet};

use sdl2::pixels::Color;

use crate::dice;
use crate::display::{DARK_BROWN, GREY, GREEN, BRIGHT_RED, BLUE, GOLD, YELLOW_ORANGE};
use crate::items::{Item, Inventory};
use crate::map;
use crate::map::Tile;
use crate::pathfinding::find_path;
use crate::ship::Ship;
use crate::util;
use crate::util::sqs_adj;

use super::{do_ability_check, GameState, Map, NPCTable};

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
	pub score: u8,
	pub poisoned: bool,
	pub charmed: bool,
	pub drunkeness: u8,
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
			score: 0,
			poisoned: false,
			charmed: false,
			drunkeness: 0,
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
			score: 0,
			poisoned: false,
			charmed: false,
			drunkeness: 0,
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

	pub fn add_stamina(&mut self, stamina: u8) {
		self.curr_stamina += stamina;
		if self.curr_stamina > self.max_stamina {
			self.curr_stamina = self.max_stamina;
		}
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
	pub special_dmg: String,
	pub score: u8,
	pub gender: u8,
	pub anchor: (usize, usize),
	pub aware_of_player: bool,
}

impl Monster {
	pub fn new(name: String, ac:u8, hp: u8, symbol: char, row: usize, col: usize, color: Color,
			hit_bonus: i8, dmg: u8, dmg_dice: u8, dmg_bonus: u8, score: u8) -> Monster {
		Monster { name, ac, hp, symbol, row, col, color, hit_bonus, 
			dmg, dmg_dice, dmg_bonus, special_dmg: String::from(""),
			gender: 0, anchor: (0, 0), score, aware_of_player: false }
	}

	pub fn new_merperson(row: usize, col: usize) -> Monster {
		let hp = dice::roll(8, 2, 0);

		let mut m = Monster::new(String::from("merperson"), 13, hp, 'y', row, col, YELLOW_ORANGE,
			5, 1, 1, 0, 10);

		m.aware_of_player = true; // they keep their eyes out for sailors

		let roll = rand::thread_rng().gen_range(0.0, 1.0);
		if roll < 0.33 {
			m.name = String::from("mermaid");
			m.gender = 1;
		} else if roll < 0.66 {
			m.name = String::from("merman");
			m.gender = 2;
		};

		m
	}

	pub fn new_pirate(row: usize, col: usize, anchor: (usize, usize)) -> Monster {
		let hp = dice::roll(8, 2, 2);

		let mut p = Monster::new(String::from("marooned pirate"), 14, hp, '@', row, col, GREY,
			5, 6, 1, 0, 10);
		p.anchor = anchor;

		let roll = rand::thread_rng().gen_range(0.0, 1.0);
		if roll < 0.33 {
			p.gender = 1;
		} else if roll < 0.66 {
			p.gender = 2;
		};
		
		p
	}
	
	pub fn new_snake(row: usize, col: usize) -> Monster {
		let hp = dice::roll(6, 2, 0);
		let roll = rand::thread_rng().gen_range(0.0, 1.0);
		
		let colour = if roll < 0.33 {
			BRIGHT_RED
		} else if roll < 0.66 {
			GOLD
		} else {
			GREEN 
		};
		
		let mut s = Monster::new(String::from("snake"), 14, hp, 'S', row, col, colour,
			4, 4, 1, 0, 10);
		s.special_dmg = String::from("poison");

		s
	}
	
	pub fn new_shark(row: usize, col: usize) -> Monster {
		let hp = dice::roll(6, 3, 0);
		Monster::new(String::from("shark"), 12, hp, '^', row, col, GREY,
			4, 8, 1, 2, 10)
	}

	pub fn new_panther(row: usize, col: usize) -> Monster {
		let hp = dice::roll(8, 4, 0);
		let mut p = Monster::new(String::from("panther"), 12, hp, 'f', row, col, BLUE,
			5, 12, 1, 2, 10);

		p.aware_of_player = true; // always on the hunt

		p
	}

	pub fn new_boar(row: usize, col: usize) -> Monster {
		let hp = dice::roll(5, 2, 0);
		Monster::new(String::from("wild boar"), 12, hp, 'b', row, col, DARK_BROWN,
			4, 6, 1, 2, 5)
	}

	// I'm sure life doesn't need to be this way, but got to figure out the
	// Rust polymorphism model
	pub fn act(&mut self, state: &mut GameState, ships: &HashMap<(usize, usize), Ship>) 
											-> Result<(), String> {
		if self.name == "shark" {
			shark_action(self, state, ships)?;
		} else if self.name == "marooned pirate" {
			pirate_action(self, state, ships)?;
		} else if self.name == "mermaid" || self.name == "merperson" || self.name == "merman" {
			merfolk_action(self, state)?;	
		} else if self.name == "wild boar" {
			basic_monster_action(self, state, ships, "gores")?;
		} else {
			basic_monster_action(self, state, ships, "bites")?;
		}

		Ok(())
	}
}

fn find_adj_empty_sq(row: i32, col: i32, state: &GameState, 
				ships: &HashMap<(usize, usize), Ship>, passable: &HashSet<map::Tile>) -> (usize, usize) {
	let mut adj = Vec::new();

	for r in -1..=1 {
		for c in -1..=1 {
			if r == 0 && c == 0 { continue; }
			let adj_r = row + r;
			let adj_c = col + c;
	
			if !map::in_bounds(&state.map, adj_r, adj_c) { continue; }
			if !passable.contains(&state.map[adj_r as usize][adj_c as usize]) { continue; }
			if !super::sq_open(state, ships, adj_r as usize, adj_c as usize) { continue; }

			adj.push((adj_r as usize, adj_c as usize));
		}
	}

	if adj.len() == 0 {
		(row as usize, col as usize)
	} else {
		let i = dice::roll(adj.len() as u8, 1, 0) - 1;
		let loc = adj[i as usize];
		loc
	}
}

fn do_special_dmg(state: &mut GameState, special_dmg: &str) {
	if special_dmg == "poison" {
		let con_mod = Player::mod_for_stat(state.player.constitution);
		if !state.player.poisoned && !do_ability_check(con_mod, 13, 0) {
			state.write_msg_buff("You are poisoned!");
			state.player.poisoned = true;
		}
	}
}

fn stealth_check(state: &mut GameState, m: &mut Monster) {
	let dex_mod = Player::mod_for_stat(state.player.dexterity);
	if super::do_ability_check(dex_mod, 13, state.player.prof_bonus as i8) {
		m.aware_of_player = true;
		state.write_msg_buff("Something snarls.");
	}
}

fn basic_monster_action(m: &mut Monster, state: &mut GameState,
							ships: &HashMap<(usize, usize), Ship>,
							verb: &str) -> Result<(), String> {
	if m.aware_of_player && sqs_adj(m.row, m.col, state.player.row, state.player.col) {
		if super::attack_player(state, m) {
			let s = format!("The {} {} you!", m.name, verb);
			state.write_msg_buff(&s);
			let dmg_roll = dice::roll(m.dmg, m.dmg_dice, m.dmg_bonus as i8);
			super::player_takes_dmg(&mut state.player, dmg_roll, &m.name)?;

			if m.special_dmg != "" {
				do_special_dmg(state, &m.special_dmg);
			}
		} else {
			let s = format!("The {} missed!", m.name);
			state.write_msg_buff(&s);
		}

		return Ok(());	
	} 

	let mut passable = HashSet::new();
	passable.insert(map::Tile::Dirt);
	passable.insert(map::Tile::Grass);
	passable.insert(map::Tile::Sand);
	passable.insert(map::Tile::Tree);
	passable.insert(map::Tile::Floor);

	let dis = util::cartesian_d(m.row, m.col, state.player.row, state.player.col);
	
	if dis > 20 {
		let loc = find_adj_empty_sq(m.row as i32, m.col as i32, state, ships, &passable);
		m.row = loc.0;
		m.col = loc.1;
	} else if !m.aware_of_player && dis < 10 {
		let loc = find_adj_empty_sq(m.row as i32, m.col as i32, state, ships, &passable);
		m.row = loc.0;
		m.col = loc.1;

		stealth_check(state, m);
	} else {
		let path = find_path(state, m.row, m.col, 
			state.player.row, state.player.col, &passable, ships);
	
		if path.len() > 1 {
			let new_loc = path[1];
			if state.npcs.contains_key(&new_loc) {
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

fn get_pirate_line() -> String {
	let roll = rand::thread_rng().gen_range(0.0, 1.0);

	if roll < 0.2 {
		return "Ye scurvy dog!".to_string();
	} else if roll < 0.4 {
		return "Arroint thee, barnacle!".to_string();
	} else if roll < 0.6 {
		return "I'll scuttle you!".to_string();
	} else if roll < 0.8 {
		return "To the locker with ye!".to_string();
	} else {
		return "I've smelled better bilges!".to_string();
	}
}

fn pirate_action(m: &mut Monster, state: &mut GameState,
					ships: &HashMap<(usize, usize), Ship>) -> Result<(), String> {
	let pronoun = if m.gender == 0 {
		"their"
	} else if m.gender == 1 {
		"her"
	} else {
		"his"
	};

	if sqs_adj(m.row, m.col, state.player.row, state.player.col) {
		if super::attack_player(state, m) {
			let s = format!("The {} slashes with {} cutlass!", m.name, pronoun);
			state.write_msg_buff(&s);
			let dmg_roll = dice::roll(m.dmg, m.dmg_dice, m.dmg_bonus as i8);
			super::player_takes_dmg(&mut state.player, dmg_roll, &m.name)?;
		} else {
			let s = format!("The {} missed!", m.name);
			state.write_msg_buff(&s);
		}	

		if rand::thread_rng().gen_range(0.0, 1.0) < 0.2 {
			state.write_msg_buff(&get_pirate_line());
		}
		
		return Ok(());
	} 

	let d = util::cartesian_d(m.row, m.col, state.player.row, state.player.col);

	if d > 20 {
		return Ok(())
	}

	if m.aware_of_player {
		// Too far away and they just ignore the player
		let mut passable = HashSet::new();
		passable.insert(map::Tile::Dirt);
		passable.insert(map::Tile::Grass);
		passable.insert(map::Tile::Water);
		passable.insert(map::Tile::Sand);
		passable.insert(map::Tile::Tree);
		passable.insert(map::Tile::Floor);

		let path = find_path(state, m.row, m.col, 
			state.player.row, state.player.col, &passable, &ships);

		let mut next_r = m.row;
		let mut next_c = m.col;
		if path.len() > 1 {
			let new_loc = path[1];
			if state.npcs.contains_key(&new_loc) {
				let s = format!("The {} is blocked.", m.name);
				state.write_msg_buff(&s);
				return Ok(());
			} 

			next_r = new_loc.0;
			next_c = new_loc.1;
		} else {
			let loc = find_adj_empty_sq(m.row as i32, m.col as i32, state, ships, &passable);
			next_r = loc.0;
			next_c = loc.1;
		}

		// The pirate won't wander too far from their campsite
		if util::cartesian_d(m.row, next_r, m.col, next_c) < 9 {
			m.row = next_r;
			m.col = next_c;
		}
	} else if d < 10 {
		stealth_check(state, m);
	}

	Ok(())
}

fn pick_fleeing_move(state: &mut GameState, m: &Monster, passable: HashSet<Tile>) -> Option<(usize, usize)> {
	// Okay, hopefully this is a decent way to do this:
	// if the monster's row < player's row, they want to keep making it smaller,
	// same with column. This will likely sometimes lead to the monster getting 
	// trapped by that's okay.
	let mut options;
	if m.row <= state.player.row && m.col <=  state.player.col {
		options = vec![(-1, -1), (-1, 0), (0, -1)];
	} else if m.row <= state.player.row && m.col > state.player.col {
		options = vec![(-1, -1), (-1, 0), (0, 1)];
	} else if m.row > state.player.row && m.col <= state.player.col {
		options = vec![(1, -1), (1, 0), (0, -1)];
	} else {
		options = vec![(1, 1), (1, 0), (0, 1)];
	} 

	let mut rng = thread_rng();
	options.shuffle(&mut rng);

	for mv in options {
		let mv_r = (m.row as i32 + mv.0) as usize;
		let mv_c = (m.col as i32 + mv.1) as usize;
		if passable.contains(&state.map[mv_r][mv_c]) 
				&& !state.npcs.contains_key(&(mv_r, mv_c)) { 
			return Some((mv_r, mv_c));
		}
	}

	None
}

// merfolk just want to lure the player to their death
fn merfolk_action(m: &mut Monster, state: &mut GameState) -> Result<(), String> {
	let dis = util::cartesian_d(m.row, m.col, state.player.row , state.player.col);
	if dis < 13 {
		if !state.player.charmed {
			state.write_msg_buff("You hear beautiful singing.");
			let verve_mod = Player::mod_for_stat(state.player.verve);

			let bonus = f32::round(state.player.drunkeness as f32 / 5.0) as i8;
			if !do_ability_check(verve_mod, 14, bonus) {
				let s = format!("You are charmed by the {}'s song!", m.name);
				state.write_msg_buff(&s);
				state.player.charmed = true;
			}
		} else if dis < 3{
			// the merperson waits for the player to approach and then swims away
			let mut water = HashSet::new();
			water.insert(map::Tile::DeepWater);
			water.insert(map::Tile::Water);

			match pick_fleeing_move(state, m, water) {
				Some(mv) => {
					m.row = mv.0;
					m.col = mv.1;
				},
				None => { return Ok(()); }
			}
		}
	} else if dis < 25 {
		// just move a random sq somteimes
		if rand::thread_rng().gen_range(0.0, 1.0) < 0.25 {
			for _ in 0..6 {
				let mv = util::rnd_adj();
				let next_r = (m.row as i32 + mv.0) as usize;
				let next_c = (m.col as i32 + mv.1) as usize;
				if state.map[next_r][next_c] == Tile::Water 
								|| state.map[next_r][next_c] == Tile::DeepWater {
					m.row = next_r;
					m.col = next_r;
					break;
				}
			}
		}
	}

	Ok(())
}

fn shark_action(m: &mut Monster, state: &mut GameState, ships: &HashMap<(usize, usize), Ship>) 
													-> Result<(), String> {
	if sqs_adj(m.row, m.col, state.player.row, state.player.col) {
		if super::attack_player(state, m) {
			state.write_msg_buff("The shark bites you!");
			let dmg_roll = dice::roll(m.dmg, m.dmg_dice, m.dmg_bonus as i8);
			super::player_takes_dmg(&mut state.player, dmg_roll, "shark")?;
		} else {
			state.write_msg_buff("The shark misses!");
		}	
	} else if util::cartesian_d(m.row, m.col, state.player.row, state.player.col) < 30 {
		// Too far away and the sharks just ignore the player
		let mut water = HashSet::new();
		water.insert(map::Tile::DeepWater);

		//println!("Shark on turn {}", state.turn);
		let path = find_path(state, m.row, m.col, 
			state.player.row, state.player.col, &water, ships);
		
		if path.len() > 1 {
			let new_loc = path[1];
			if state.npcs.contains_key(&new_loc) {
				let s = format!("The {} is blocked.", m.name);
				state.write_msg_buff(&s);
				return Ok(());
			} 

			m.row = new_loc.0;
			m.col = new_loc.1;
		} else {
			let loc = find_adj_empty_sq(m.row as i32, m.col as i32, state, ships, &water);
			m.row = loc.0;
			m.col = loc.1;
		}
	}

	Ok(())
}

