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
extern crate sdl2;

mod actor;
mod content_factory;
mod dice;
mod display;
mod fov;
mod items;
#[allow(dead_code)]
mod map;
#[allow(dead_code)]
mod pathfinding;
mod ship;
mod util;

use crate::actor::{Monster, Player, PirateType};
use crate::content_factory::generate_world;
use crate::display::{GameUI, SidebarInfo};
use crate::items::{Item, ItemType, ItemsTable};
use crate::map::Tile;
use crate::pathfinding::find_path;
use crate::ship::Ship;

use rand::Rng;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

const MSG_HISTORY_LENGTH: usize = 50;
const FOV_WIDTH: usize = 41;
const FOV_HEIGHT: usize = 21;

pub type Map = Vec<Vec<map::Tile>>;
type NPCTable = HashMap<(usize, usize), Monster>;

pub enum Cmd {
	Quit,
	Move(String),
	MsgHistory,
	PickUp,
	ShowInventory,
	DropItem,
	ShowCharacterSheet,
	ToggleEquipment,
	Pass,
	TurnWheelClockwise,
	TurnWheelAnticlockwise,	
	ToggleAnchor,
	ToggleHelm,
	Quaff,
	FireGun,
	Reload,
	WorldMap,
	Search,
	Read,
	Eat,
}

pub struct GameState {
	player: Player,
	msg_buff: VecDeque<String>,
	msg_history: VecDeque<(String, u32)>,
	map: Map,
	npcs: NPCTable,
	turn: u32,
	world_seen: HashSet<(usize, usize)>,
	pirate_lord: String,
	pirate_lord_ship: String,
	player_ship: String,
	starter_clue: u8,
	notes: HashMap<u8, String>,
	note_count: u8,
}

impl GameState {
	pub fn new_pirate(name: String, p_type: PirateType, npcs: NPCTable) -> GameState {
		let player = match p_type {
			PirateType::Swab => Player::new_swab(name),
			PirateType::Seadog => Player::new_seadog(name),
		};

		GameState {player, msg_buff: VecDeque::new(), 
			msg_history: VecDeque::new(), turn: 0, map: Vec::new(), npcs,
			world_seen: HashSet::new(), pirate_lord: String::from(""),
			player_ship: String::from(""), pirate_lord_ship: String::from(""),
			starter_clue: 0, notes: HashMap::new(), note_count: 0 
		}
	}

	pub fn curr_sidebar_info(&self) -> SidebarInfo {
		let bearing: i8;
		let wheel: i8;  
		
		if self.player.on_ship {
			bearing = self.player.bearing as i8;
			wheel = self.player.wheel;
		} else {
			bearing = -1;
			wheel = -1;
		};

		SidebarInfo::new(self.player.name.clone(), self.player.ac,
			self.player.curr_stamina, self.player.max_stamina, wheel, bearing, self.turn,
			self.player.charmed, self.player.poisoned, self.player.drunkeness)
	}

	pub fn write_msg_buff(&mut self, msg: &str) {
		let s = String::from(msg);
		self.msg_buff.push_back(s);

		if msg.len() > 0 {
			if self.msg_history.len() == 0 || msg != self.msg_history[0].0 {
				self.msg_history.push_front((String::from(msg), 1));
			} else {
				self.msg_history[0].1 += 1;
			}

			if self.msg_history.len() > MSG_HISTORY_LENGTH {
				self.msg_history.pop_back();
			}
		}
	}
}

fn sq_is_open(state: &GameState, ships: &HashMap<(usize, usize), Ship>,
											  row: usize, col: usize) -> bool {
	if state.player.row == row && state.player.col == col {
		return false;
	} 

	if state.npcs.contains_key(&(row, col)) {
		return false;
	}

	// Ships complicate EVERYTHING T_T. I almost need a master hash table of like
	// Pieces that contains both monsters and ship parts that I can update when
	// things move, but Rust's borrow rules and lack of polymorphism would turn it
	// into a horrific mess to code
	let ship_locs = ships.keys()
					.map(|s| s.clone())
					.collect::<Vec<(usize, usize)>>();

	for sl in ship_locs {
		if util::cartesian_d(row, col, sl.0, sl.1) < 2 {
			if row == sl.0 && col == sl.1 {
				return false;
			}
			let ship = ships.get(&(sl.0, sl.1)).unwrap();
			if row == ship.bow_row && col == ship.bow_col {
				return false;
			}
			if row == ship.aft_row && col == ship.aft_col {
				return false;
			}
		}
	}

	true
}
 
fn get_move_tuple(mv: &str) -> (i32, i32) {
	let res: (i32, i32);

  	if mv == "N" {
		res = (-1, 0)
	} else if mv == "S" {
		res = (1, 0)
	} else if mv == "W" {
		res = (0, -1)
	} else if mv == "E" {
		res = (0, 1)
	} else if mv == "NW" {
		res = (-1, -1)
	} else if mv == "NE" {
		res = (-1, 1)
	} else if mv == "SW" {
		res = (1, -1)
	} else {
		res = (1, 1)
	}

	res
}

fn do_ability_check(ability_mod: i8, difficulty: u8, bonus: i8) -> bool {
	let roll = dice::roll(20, 1, 0) as i8 + ability_mod + bonus;

	if roll >= difficulty as i8 {
		true
	} else {
		false
	}
}

fn player_takes_dmg(player: &mut Player, dmg: u8, source: &str) -> Result<(), String> {
	if player.curr_stamina < dmg {
		Err(source.to_string())
	} else {
		player.curr_stamina -= dmg;
		Ok(())
	}
}

fn attack_npc(state: &mut GameState, npc_row: usize, npc_col: usize) {
	let mut npc = state.npcs.remove(&(npc_row, npc_col)).unwrap();
	npc.aware_of_player = true;
	let str_mod = Player::mod_for_stat(state.player.strength);

	if do_ability_check(str_mod, npc.ac, state.player.prof_bonus as i8) {
		let mut dmg: i8;
		match state.player.inventory.get_equiped_weapon() {
			Some(w) => {
				let s = format!("You hit the {}!", npc.name);
				state.write_msg_buff(&s);
				dmg = dice::roll(w.dmg, w.dmg_dice, w.bonus as i8) as i8 + str_mod;
			},
			None => {
				let s = format!("You punch the {}!", npc.name);
				state.write_msg_buff(&s);
				dmg = 1 + str_mod;
			}
		}

		// It could happen??	
		if dmg < 0 {
			dmg = 0;
		}

		if dmg as u8 > npc.hp {
			let s = format!("You kill the {}!", npc.name);
			state.write_msg_buff(&s);
			state.player.score += npc.score;
			if npc.score > 0 {
				state.player.max_stamina += 1;
			}
		} else {
			npc.hp -= dmg as u8;
			state.npcs.insert((npc_row, npc_col), npc);
		}
	} else {
		let s = format!("You miss the {}!", npc.name);
		state.write_msg_buff(&s);
		state.npcs.insert((npc_row, npc_col), npc);
	}

	state.turn += 1;
}

fn calc_bullet_ch(dir: (i32, i32)) -> char {
	if dir == (0, -1)  || dir == (0, 1)  { return '-'; }
	if dir == (1, 0)   || dir == (-1, 0) { return '|'; }
	if dir == (-1, -1) || dir == (1, 1)  { return '\\'; }

	('/')
}

fn shoot(state: &mut GameState, dir: (i32, i32), gun: &Item, dex_mod: i8, gui: &mut GameUI,
			items: &ItemsTable, ships: &HashMap<(usize, usize), Ship>) {
	let mut bullet_r = state.player.row as i32;
	let mut bullet_c = state.player.col as i32;
	let mut distance = 0;
	let mut travelled = (0, 0);

	loop {
		bullet_r += dir.0;
		bullet_c += dir.1;
		travelled = (travelled.0 + dir.0, travelled.1 + dir.1);
		distance += 1;

		if !map::in_bounds(&state.map, bullet_r, bullet_c) { break; }
		if !map::is_passable(&state.map[bullet_r as usize][bullet_c as usize]) { break; }
		if distance > gun.range { break; }

		// Sophisticated animation goes here!
		gui.v_matrix = fov::calc_v_matrix(state, items, ships, FOV_HEIGHT, FOV_WIDTH);
		// Okay, need to calcuate where in the v_matrix the bullet currently is
		let bullet_tile_r = (gui.v_matrix.len() / 2) as i32 + travelled.0;
		let bullet_tile_c = (gui.v_matrix[0].len() / 2) as i32 + travelled.1;

		// note, not currently checked for bounds because firearms don't have a range > screen dimensions...
		if gui.v_matrix[bullet_tile_r as usize][bullet_tile_c as usize] != map::Tile::Blank {
			let ch = calc_bullet_ch(dir);
			gui.v_matrix[bullet_tile_r as usize][bullet_tile_c as usize] = map::Tile::Bullet(ch);
		}
		let sbi = state.curr_sidebar_info();
		gui.write_screen(&mut state.msg_buff, &sbi);
		// probably need to pause here, or I guess not because my frame drawing is so slow...

		if state.npcs.contains_key(&(bullet_r as usize, bullet_c as usize)) {
			let mut npc = state.npcs.remove(&(bullet_r as usize, bullet_c as usize)).unwrap();
			if do_ability_check(dex_mod, npc.ac, state.player.prof_bonus as i8) {
				let s = format!("Your bullet hits the {}", npc.name);
				state.write_msg_buff(&s);

				let mut dmg = dice::roll(gun.dmg, gun.dmg_dice, gun.bonus as i8) as i8 + dex_mod;

				npc.hostile = true;
				npc.aware_of_player = true;

				// The damanging npc code is duplicated from the attack_npc() method
				// so maybe extract into a separate function?
				if dmg < 0 {
					dmg = 0;
				}

				if dmg as u8 > npc.hp {
					let s = format!("You kill the {}!", npc.name);
					state.write_msg_buff(&s);
					state.player.score += npc.score;
					return; // return here so the npc doesn't get added back into the npc structure
				} else {
					npc.hp -= dmg as u8;
				}

				state.npcs.insert((bullet_r as usize, bullet_c as usize), npc);
				break; // We hit someone so the bullet stops
			} else {
				state.npcs.insert((bullet_r as usize, bullet_c as usize), npc);
			}
		}
	}
}

fn fire_gun(state: &mut GameState, gui: &mut GameUI, items: &ItemsTable, 
			ships: &HashMap<(usize, usize), Ship>) {
	let dex_mod = Player::mod_for_stat(state.player.dexterity);

	match state.player.inventory.get_equiped_firearm() {
		Some(g) => {
			if g.loaded {
				let sbi = state.curr_sidebar_info();
				match gui.pick_direction(&sbi) {
					Some(dir) => { 
						state.write_msg_buff("Bang!");
						shoot(state, dir, &g, dex_mod, gui, items, ships);
						state.turn += 1;
					},
					None => state.write_msg_buff("Nevermind."),
				}
				state.player.inventory.firearm_fired();
			} else {
				state.write_msg_buff("Click, click.");
				state.turn += 1;
			}
		},
		None => state.write_msg_buff("You don't have a firearm ready."),
	}
}

fn action_while_charmed(state: &mut GameState, items: &ItemsTable, 
			ships: &HashMap<(usize, usize), Ship>) -> Result<(), String> {
	// the charmed player attempts to swim to the mermaid
	if state.player.on_ship {
		state.player.on_ship = false;
		state.write_msg_buff("You walked away from the helm.");
		state.turn += 1;
		return Ok(());
	} 

	let mut nearest = 999;
	let mut best = (0, 0);
	for r in -12..12 {
		for c in -12..12 {
			let sq_r = (state.player.row as i32 + r) as usize;
			let sq_c = (state.player.col as i32 + c) as usize;
			if state.npcs.contains_key(&(sq_r, sq_c)) { 
				let m = &state.npcs[&(sq_r, sq_c)];
				if m.name == "mermaid" || m.name == "merman" || m.name == "merperson" {
					let d = util::cartesian_d(state.player.row, state.player.col, sq_r, sq_c);
					if d < nearest {
						nearest = d;
						best = ((r + state.player.row as i32) as usize, 
								(c + state.player.col as i32) as usize);
					}
				}			
			} 
		}
	}

	if nearest > 1 && best != (0, 0) {
		let passable = map::all_passable();
		let path = find_path(state, state.player.row, state.player.col,
			best.0, best.1, &passable, ships);

		if path.len() > 1 {
			let mv = &path[1];
			state.write_msg_buff("You are drawn to the merfolk!");
			let dir = util::dir_between_sqs(state.player.row, state.player.col, mv.0, mv.1);
			do_move(state, items, ships, &dir)?;
		}
	}
	else {
		state.write_msg_buff("You are entranced by the merfolk!");
		state.turn += 1;
	}

	Ok(())
}

fn check_environment_hazards(state: &mut GameState) -> Result<(), String> {
	let pr = state.player.row;
	let pc = state.player.col;

	if state.map[pr][pc] == Tile::DeepWater && !state.player.on_ship {
		player_takes_dmg(&mut state.player, 2, "swimming")?;
	} else if state.map[pr][pc] == Tile::FirePit {
		let dmg = dice::roll(6, 1, 0);
		player_takes_dmg(&mut state.player, dmg, "burn")?;
	} else if state.map[pr][pc] == Tile::Lava {
		player_takes_dmg(&mut state.player, 25, "burn")?;
	}

	Ok(())
}

fn do_move(state: &mut GameState, items: &ItemsTable, 
			ships: &HashMap<(usize, usize), Ship>, dir: &str) -> Result<(), String> {
	let mut mv = get_move_tuple(dir);

	// if the player is poisoned they'll sometimes stagger
	if state.player.poisoned || state.player.drunkeness > 20 {
		if rand::thread_rng().gen_range(0.0, 1.0) < 0.25 {
			state.write_msg_buff("You stagger!");
			mv = util::rnd_adj();
		}
	}

	let start_tile = &state.map[state.player.row][state.player.col];
	let next_row = state.player.row as i32 + mv.0;
	let next_col = state.player.col as i32 + mv.1;
	let next_loc = (next_row as usize, next_col as usize);
	let tile = &state.map[next_row as usize][next_col as usize];
	
	if state.npcs.contains_key(&next_loc) {
		attack_npc(state, next_loc.0, next_loc.1);
	} else if ships.contains_key(&next_loc) {
		state.player.col = next_col as usize;
		state.player.row = next_row as usize;
		let ship = ships.get(&next_loc).unwrap();
		let s = format!("You climb aboard the {}.", ship.name);
		state.write_msg_buff(&s);
		state.turn += 1;
	} else if map::is_passable(tile) {
		state.player.col = next_col as usize;
		state.player.row = next_row as usize;

		match tile {
			map::Tile::Water => state.write_msg_buff("You splash in the shallow water."),
			map::Tile::DeepWater => {
				if *start_tile != map::Tile::DeepWater {
					state.write_msg_buff("You begin to swim.");				
				}

				if state.player.curr_stamina < 10 {
					state.write_msg_buff("You're getting tired...");
				}
			},
			map::Tile::Lava => state.write_msg_buff("MOLTEN LAVA!"),
			map::Tile::FirePit => {
				state.write_msg_buff("You step in the fire!");
			},
			map::Tile::Shipwreck(_, name) => {
				let s = format!("The wreck of the {}", name);
				state.write_msg_buff(&s);
			},
			map::Tile::OldFirePit => state.write_msg_buff("An old campsite! Rum runners? A castaway?"),
			_ => {
				if *start_tile == map::Tile::DeepWater && state.player.curr_stamina < 10 {
					state.write_msg_buff("Whew, you stumble ashore.");
				}
			},
		}

		let items_count = items.count_at(state.player.row, state.player.col);
		if items_count == 1 {
			let i = items.peek_top(state.player.row, state.player.col);
			let s = format!("You see {} here.", util::get_articled_name(false, &i));
			state.write_msg_buff(&s);
		} else if items_count > 1 {
			state.write_msg_buff("You see a few items here.");
		}	

		state.turn += 1;
	} else  {
		state.write_msg_buff("You cannot go that way.");
	}

	Ok(())
}

fn show_message_history(state: &GameState, gui: &mut GameUI) {
	let mut lines = Vec::new();
	lines.push("".to_string());
	for j in 0..state.msg_history.len() {
		let mut s = state.msg_history[j].0.to_string();
		if state.msg_history[j].1 > 1 {
			s.push_str(" (x");
			s.push_str(&state.msg_history[j].1.to_string());
			s.push_str(")");
		}
		lines.push(s);
	}

	gui.write_long_msg(&lines, true);
}

// Attempt to reasonably pluralize names
// I'm going to assume a fairly standard form of names of things that
// can be pluralized. Like, "foo of bar" so I can asssume the result will
// be foos of bar.
fn pluralize(name: &str) -> String{
	let mut result = String::from("");
	let words = name.split(' ').collect::<Vec<&str>>();
	
	if words.len() == 1 {
		result.push_str(name);
		if name.ends_with("s") || name.ends_with("x") {
			result.push_str("es");
		} else {
			result.push_str("s");
		}
	} else {
		result.push_str(words[0]);
		if words[0].ends_with("s") || words[0].ends_with("x") {
			result.push_str("es");
		} else {
			result.push_str("s");
		}
		
		for w in 1..words.len() {
			result.push(' ');
			result.push_str(words[w]);
		}
	}

	result	
}

fn consume_nourishment(state: &mut GameState, item: &Item) {
	let hp = dice::roll(item.bonus, 1, 0);
	state.player.add_stamina(hp);

	if item.name == "draught of rum" {
		state.write_msg_buff("You drink some rum.");
		state.player.drunkeness += 10;
	} else if item.name == "coconut" {
		state.write_msg_buff("Munch munch.");
	} else if item.name == "banana" {
		state.write_msg_buff("Munch munch.");
	} else if item.name == "salted pork" {
		state.write_msg_buff("Not very satisfying.");
	}
}

fn quaff(state: &mut GameState, gui: &mut GameUI) {
	if state.player.inventory.get_menu().len() == 0 {
		state.write_msg_buff("You are empty handed.");
		return
	}

	let sbi = state.curr_sidebar_info();
	match gui.query_single_response("Quaff what?", &sbi) {
		Some(ch) => {
			match state.player.inventory.item_type_in_slot(ch) {	
				Some(ItemType::Drink) => {
					let drink = state.player.inventory.remove_count(ch, 1);
					consume_nourishment(state, &drink[0]);
					state.turn += 1;
				},
				Some(_) => state.write_msg_buff("Uh...ye can't drink that."),
				None => state.write_msg_buff("You do not have that item."),
			}
		},
		None => state.write_msg_buff("Nevermind."),
	}
}

fn eat(state: &mut GameState, gui: &mut GameUI) {
	if state.player.inventory.get_menu().len() == 0 {
		state.write_msg_buff("You are empty handed.");
		return
	}

	let sbi = state.curr_sidebar_info();
	match gui.query_single_response("Eat what?", &sbi) {
		Some(ch) => {
			match state.player.inventory.item_type_in_slot(ch) {	
				Some(ItemType::Food) => {
					let food = state.player.inventory.remove_count(ch, 1);
					consume_nourishment(state, &food[0]);
					state.turn += 1;
				},
				Some(_) => state.write_msg_buff("Uh...ye can't eat that."),
				None => state.write_msg_buff("You do not have that item."),
			}
		},
		None => state.write_msg_buff("Nevermind."),
	}
}

fn read(state: &mut GameState, gui: &mut GameUI) {
	if state.player.inventory.get_menu().len() == 0 {
		state.write_msg_buff("You are empty handed.");
		return
	}
	
	let sbi = state.curr_sidebar_info();
	match gui.query_single_response("Read what?", &sbi) {
		Some(ch) => {
			match state.player.inventory.item_type_in_slot(ch) {	
				Some(ItemType::TreasureMap) => {
					let map = state.player.inventory.peek_at(ch).unwrap();
					gui.show_treasure_map(state, &map);
					state.turn += 1;
				},
				Some(ItemType::Note) => {
					let note = state.player.inventory.peek_at(ch).unwrap();
					let txt = state.notes[&note.bonus].clone();
					state.write_msg_buff(&txt);
					state.turn += 1;
				},
				Some(_) => state.write_msg_buff("Hmm...nary a label nor instructions."),
				None => state.write_msg_buff("You do not have that item."),
			}
		},
		None => state.write_msg_buff("Nevermind."),
	}
}

fn search(state: &mut GameState, items: &mut ItemsTable) {
	let loc = (state.player.row, state.player.col);

	// For the final treasure type, a MacGuffin can only be found if the player
	// is wearing the magic eye patch	
	let mut search_dc = 15;
	if items.macguffin_here(&loc) {
		 if state.player.inventory.equiped_magic_eye_patch() {
			search_dc = 0;
		} else {
			search_dc = 99;
		}
	}

	if items.any_hidden(&loc) && do_ability_check(0, search_dc, state.player.prof_bonus as i8) {
		// hmm I wonder if I should give the player a perception skill?
		// also should have a way to have harder to find things
		state.write_msg_buff("You find a hidden cache!");
		items.reveal_hidden(&loc);
	} else if items.count_at(state.player.row, state.player.col) > 0 {
		state.write_msg_buff("You find no secrets.");
	} else {
		state.write_msg_buff("You find nothing.");
	}

	state.turn += 1;
}

fn reload(state: &mut GameState, gui: &mut GameUI) {
	match state.player.inventory.get_equiped_firearm() {
		Some(g) => {
			if g.loaded {
				let s = format!("Your {} is already loaded.", g.name);
				state.write_msg_buff(&s);
			} else if state.player.inventory.find_ammo() {
				let s = format!("You reload your {}", g.name);
				state.write_msg_buff(&s);
				state.player.inventory.reload_firearm();
			} else {
				state.write_msg_buff("Uhoh, all out of bullets...");
			}
			state.turn += 1;
		},
		None => state.write_msg_buff("You don't have a readied firearm."),
	}	
}

fn drop_item(state: &mut GameState, items: &mut ItemsTable, gui: &mut GameUI) {
	if state.player.inventory.get_menu().len() == 0 {
		state.write_msg_buff("You are empty handed.");
		return
	}

	let sbi = state.curr_sidebar_info();

	match gui.query_single_response("Drop what?", &sbi) {
		Some(ch) =>  {
			let count = state.player.inventory.count_in_slot(ch);
			if count == 0 {
				state.write_msg_buff("You do not have that item.");
			} else if count > 1 {
				match gui.query_natural_num("Drop how many?", &sbi) {
					Some(v) => {
						let pile = state.player.inventory.remove_count(ch, v);
						if pile.len() > 0 {
							let pluralized = pluralize(&pile[0].name);
							let s = format!("You drop {} {}", v, pluralized);
							state.write_msg_buff(&s);
							state.turn += 1;
							for mut item in pile {
								item.equiped = false;
								items.add(state.player.row, state.player.col, item);
							}
						} else {
							state.write_msg_buff("Nevermind.");
						}
					},
					None => state.write_msg_buff("Nevermind."),
				}
			} else {
				let mut item = state.player.inventory.remove(ch);
				item.equiped = false;
				let s = format!("You drop the {}.", util::get_articled_name(true, &item));
				items.add(state.player.row, state.player.col, item);	
				state.write_msg_buff(&s);
				state.turn += 1;
			}	
		},
		None => state.write_msg_buff("Nevermind."),
	}

	state.player.calc_ac();
}

fn pick_up(state: &mut GameState, items: &mut ItemsTable, gui: &mut GameUI) -> Result<(), String> {
	let item_count = items.count_at(state.player.row, state.player.col);
	if item_count == 0 {
		state.write_msg_buff("There is nothing here to pick up.");
	} else if item_count == 1 {
		let item = items.get_at(state.player.row, state.player.col);
		let is_macguffin = item.item_type == ItemType::MacGuffin;
		let s = format!("You pick up {}.", util::get_articled_name(true, &item));
		state.write_msg_buff(&s);
		state.player.inventory.add(item);
		state.turn += 1;

		if is_macguffin {
			return Err(String::from("victory!"));
		}
	} else {
		let mut menu = items.get_menu(state.player.row, state.player.col);
		menu.insert(0, "Pick up what: (* to get everything)".to_string());
		let answers = gui.menu_picker(&menu, menu.len() as u8, false, false);
		match answers {
			None => state.write_msg_buff("Nevermind."), // Esc was pressed
			Some(v) => {
				state.turn += 1;
				let picked_up = items.get_many_at(state.player.row, state.player.col, &v);
				for item in picked_up {
					let is_macguffin = item.item_type == ItemType::MacGuffin;
					let article = item.get_definite_article();
					let s = format!("You pick up {}.", util::get_articled_name(true, &item));
					state.write_msg_buff(&s);
					state.player.inventory.add(item);
				
					if is_macguffin {
						return Err(String::from("victory!"));
					}
				}
			},
		}
	}

	Ok(())
}

fn toggle_equipment(state: &mut GameState, gui: &mut GameUI) {
	if state.player.inventory.get_menu().len() == 0 {
		state.write_msg_buff("You are empty handed.");
		return
	}

	let sbi = state.curr_sidebar_info();
	match gui.query_single_response("Ready/unready what?", &sbi) {
		Some(ch) => {
			let result = state.player.inventory.toggle_slot(ch);
			state.write_msg_buff(&result);
			state.turn += 1;
		},
		None => state.write_msg_buff("Nevermind."),
	}

	state.player.calc_ac();
}

fn show_inventory(state: &mut GameState, gui: &mut GameUI) {
	let mut menu = state.player.inventory.get_menu();

	if menu.len() == 0 {
		state.write_msg_buff("You are empty-handed.");
	} else {
		menu.insert(0, "You are carrying:".to_string());
		gui.write_long_msg(&menu, false);
	}
}

fn show_character_sheet(state: &GameState, gui: &mut GameUI) {
	let s = format!("{}, a bilge rat", state.player.name);
	let mut lines = vec![s];
	lines.push("".to_string());
	let s = format!("Strength: {}", state.player.strength);
	lines.push(s);
	let s = format!("Dexterity: {}", state.player.dexterity);
	lines.push(s);
	let s = format!("Constitution: {}", state.player.constitution);
	lines.push(s);
	let s = format!("Verve: {}", state.player.verve);
	lines.push(s);
	lines.push("".to_string());
	let s = format!("AC: {}    Stamina: {}({})", state.player.ac, state.player.curr_stamina, state.player.max_stamina);
	lines.push(s);

	gui.write_long_msg(&lines, true);
}

// This function is a testament to my terrible design mistakes :( I should have taken into account the
// need to see if a square was open when choosing to use separate data structures for the map tiles, 
// the items, the ships, and the NPCs...
fn sq_open(state: &GameState, ships: &HashMap<(usize, usize), Ship>, row: usize, col: usize) -> bool {
	if !map::in_bounds(&state.map, row as i32, col as i32) {
		return false;
	}

	if !map::is_passable(&state.map[row][col]) {
		return false;
	}

	// At least I will probably only ever have a handful of ships on the map...
	for key in ships.keys() {
		let ship = ships.get(key).unwrap();
		if ship.row == row && ship.col == col {
			return false;
		}	
		if ship.bow_row == row && ship.bow_col == col {
			return false;
		}	
		if ship.aft_row == row && ship.aft_col == col {
			return false;
		}	
	}

	true
}

fn get_open_sq_adj_player(state: &GameState, ships: &HashMap<(usize, usize), Ship>) -> Option<(usize, usize)> {
	let mut sqs: Vec<(usize, usize)> = Vec::new();
	if sq_open(state, ships, state.player.row - 1, state.player.col - 1) {
		sqs.push((state.player.row - 1, state.player.col - 1));
	}
	if sq_open(state, ships, state.player.row - 1, state.player.col) {
		sqs.push((state.player.row - 1, state.player.col));
	}
	if sq_open(state, ships, state.player.row - 1, state.player.col + 1) {
		sqs.push((state.player.row - 1, state.player.col + 1));
	}
	if sq_open(state, ships, state.player.row, state.player.col + 1) {
		sqs.push((state.player.row, state.player.col + 1));
	}
	if sq_open(state, ships, state.player.row, state.player.col - 1) {
		sqs.push((state.player.row, state.player.col - 1));
	}
	if sq_open(state, ships, state.player.row + 1, state.player.col - 1) {
		sqs.push((state.player.row + 1, state.player.col - 1));
	}
	if sq_open(state, ships, state.player.row + 1, state.player.col) {
		sqs.push((state.player.row + 1, state.player.col));
	}
	if sq_open(state, ships, state.player.row + 1, state.player.col + 1) {
		sqs.push((state.player.row + 1, state.player.col + 1));
	}

	if sqs.len() == 0 {
		None
	} else {
		let j = (dice::roll(sqs.len() as u8, 1, 0) - 1) as usize;
		let loc = sqs[j];
		Some(loc)
	}
}

fn ship_hit_land(state: &mut GameState, ship: &mut Ship, ships: &HashMap<(usize, usize), Ship>) -> Result<(), String> {
	state.write_msg_buff("Ye've run yer ship aground!!");
	state.write_msg_buff("You lose control o' the wheel!");
	let mut new_wheel = ship.wheel + 2 + dice::roll(5, 1, 0) as i8;	
	new_wheel = new_wheel % 5 - 2;
	ship.wheel = new_wheel;
	state.player.wheel = new_wheel;

	if !do_ability_check(Player::mod_for_stat(state.player.dexterity), 13, 0) {
		if let Some(loc)= get_open_sq_adj_player(state, ships) {
			state.write_msg_buff("You're tossed from the ship!");
			state.player.on_ship = false;
			state.player.row = loc.0;
			state.player.col = loc.1;

			let dmg = dice::roll(6, 1, 0);
			player_takes_dmg(&mut state.player, dmg, "falling")?;
		}
	}

	Ok(())
}

fn sail(state: &mut GameState, ships: &mut HashMap<(usize, usize), Ship>) -> Result<(), String> {
	let mut ship = ships.remove(&(state.player.row, state.player.col)).unwrap();

	let bow_tile = state.map[ship.bow_row][ship.bow_col].clone();
	let ship_tile = state.map[ship.row][ship.col].clone();

	if ship.anchored {
		state.write_msg_buff("The ships bobs.");
	} else if bow_tile != map::Tile::Water && bow_tile != map::Tile::DeepWater {
		state.write_msg_buff("Your ship is beached!");
	} else { 
		let mut delta: (i8, i8) = (0, 0);
		if ship.bearing == 0 {
			delta = (-1, 0);
		} else if ship.bearing == 1 {
			if ship.prev_move == (-1, 0) {
				delta = (-1, 1);
			} else {
				delta = (-1, 0);
			}
		} else if ship.bearing == 2 {
			delta = (-1, 1);
		} else if ship.bearing == 3 {
			if ship.prev_move == (-1, 1) {
				delta = (0, 1);
			} else {
				delta = (-1, 1);
			}
		} else if ship.bearing == 4 {
			delta = (0, 1);
		} else if ship.bearing == 5 {
			if ship.prev_move == (0, 1) {
				delta = (1, 1);
			} else {
				delta = (0, 1);
			}
		} else if ship.bearing == 6 {
			delta = (1, 1);
		} else if ship.bearing == 7 { 
			if ship.prev_move == (1, 1) {
				delta = (1, 0);
			} else {
				delta = (1, 1);
			}
		} else if ship.bearing == 8 {
			delta = (1, 0);
		} else if ship.bearing == 9 {
			if ship.prev_move == (1, -1) {
				delta = (1, 0);
			} else {
				delta = (1, -1);
			}
		} else if ship.bearing == 10 {
			delta = (1, -1);
		} else if ship.bearing == 11 {
			if ship.prev_move == (0, -1) {
				delta = (1, -1);
			} else {
				delta = (0, -1);
			}
		} else if ship.bearing == 12 {
			delta = (0, -1);
		} else if ship.bearing == 13 {
			if ship.prev_move == (0, -1) {
				delta = (-1, -1);
			} else {
				delta = (0, -1);
			}
		} else if ship.bearing == 14 {
			delta = (-1, -1);
		} else if ship.bearing == 15 {
			if ship.prev_move == (-1, 0) {
				delta = (-1, -1);
			} else {
				delta = (-1, 0);
			}
		}

		// after movement, if the wheel is turned, adjust the bearing /*
		if ship.wheel != 0 {
			let mut new_bearing = ship.bearing as i8 + ship.wheel;
			
			// Ugh how I wish that Rust handled -1 % 16 == 15 like Python does
			// instead of returning -1...
			if new_bearing < 0 {
				new_bearing = 16 + ship.wheel;
			} else if new_bearing > 15 {
				new_bearing = 0 + ship.wheel;
			}

			ship.bearing = new_bearing as u8;
			state.player.bearing = new_bearing as u8;
		}

		state.player.row = (state.player.row as i32+ delta.0 as i32) as usize;
		state.player.col = (state.player.col as i32 + delta.1 as i32) as usize;
		ship.row = (ship.row as i32 + delta.0 as i32) as usize;
		ship.col = (ship.col as i32 + delta.1 as i32) as usize;
		ship.update_loc_info();
		ship.prev_move = delta;

		//if map[ship.row][ship.col] == map::Tile::Water || 
		if state.map[ship.bow_row][ship.bow_col] == map::Tile::Water {
			state.write_msg_buff("Shallow water...");
		} else if state.map[ship.bow_row][ship.bow_col] != map::Tile::DeepWater {
			ship_hit_land(state, &mut ship, ships)?;
		}
	}

	ships.insert((ship.row, ship.col), ship);

	Ok(())
}

fn toggle_anchor(state: &mut GameState, ships: &mut HashMap<(usize, usize), Ship>) -> bool {
	let mut ship = ships.get_mut(&(state.player.row, state.player.col)).unwrap();
	ship.anchored = !ship.anchored;

	state.turn += 1;

	if ship.anchored {
		state.write_msg_buff("You lower the anchor.");
		false
	} else {
		state.write_msg_buff("You raise the anchor.");
		true
	}
}

fn turn_wheel(state: &mut GameState, ships: &mut HashMap<(usize, usize), Ship>, change: i8) {
	let mut ship = ships.get_mut(&(state.player.row, state.player.col)).unwrap();

	state.turn += 1;
	if change < 0 && ship.wheel == -2 {
		state.write_msg_buff("The wheel's as far to starboard as she'll turn");
		return;
	} else if change > 0 && ship.wheel == 2 {
		state.write_msg_buff("The wheel's as far to port as she'll turn");
		return;
	}

	ship.wheel += change;
	state.player.wheel = ship.wheel;

	if ship.wheel > -2 && ship.wheel < 2 {
		state.write_msg_buff("You adjust the tiller.");
	} else {
		state.write_msg_buff("Hard about!");
	}
}

fn take_helm(state: &mut GameState, ships: &HashMap<(usize, usize), Ship>) {
	let player_loc = (state.player.row, state.player.col);
	if !ships.contains_key(&player_loc) {
		state.write_msg_buff("You need to find yerself a ship before you can take the helm.");
		return;
	}

	let ship = ships.get(&player_loc).unwrap();
	state.player.on_ship = true;
	state.player.bearing = ship.bearing;
	state.player.wheel = ship.wheel;
	
	let s = format!("You step to the wheel of the {}.", ship.name);
	state.write_msg_buff(&s);

	state.turn += 1;
}

fn leave_helm(state: &mut GameState) {
	state.player.on_ship = false;
	state.write_msg_buff("You step to gunwale.");
	state.turn += 1;
}

fn title_screen(gui: &mut GameUI) {
	let mut lines = vec!["Welcome to YarrL, a roguelike adventure on the high seas!".to_string(), "".to_string()];
	lines.push("".to_string());
	lines.push("".to_string());
	lines.push("  I must down to the seas again,".to_string());
	lines.push("      to the lonely sea and the sky,".to_string());
	lines.push("  And all I ask is a tall ship".to_string()); 
	lines.push("      and a star to steer her by,".to_string());
	lines.push("  And the wheel’s kick and the wind’s song".to_string()); 
	lines.push("      and the white sail’s shaking,".to_string());
	lines.push("  And a grey mist on the sea’s face,".to_string());
	lines.push("      and a grey dawn breaking.".to_string());
	lines.push("".to_string());
	lines.push("                     -- Sea Fever, John Masefield".to_string());
	lines.push("".to_string());
	lines.push("".to_string());
	lines.push("".to_string());
	lines.push("".to_string());
	lines.push("YarrL is copyright 2020 by Dana Larose, see COPYING for licence info.".to_string());
	
	gui.write_long_msg(&lines, true);
}

fn confirm_quit(state: &GameState, gui: &mut GameUI) -> Result<(), String> {
	let sbi = state.curr_sidebar_info();
	match gui.query_yes_no("Do you really want to Quit? (y/n)", &sbi) {
		'y' => Err("quit".to_string()),
		_ => Ok(()),
	}
}

fn is_putting_on_airs(name: &str) -> bool {
	name.to_lowercase().starts_with("capt") ||
		name.to_lowercase().starts_with("capn") ||
		name.to_lowercase().starts_with("cap'n") 
}

fn preamble(gui: &mut GameUI, ships: &mut HashMap<(usize, usize), Ship>) -> GameState {
	let mut player_name: String;

	let sbi = SidebarInfo::new("".to_string(), 0, 0, 0, -1, -1, 0, false, false, 0);
	loop {
		if let Some(name) = gui.query_user("Ahoy lubber, who be ye?", 15, &sbi) {
			if name.len() > 0 {
				player_name = name;

				if is_putting_on_airs(&player_name) {
					let v = vec![String::from("Don't ye be calling yerself *captain* 'afore"), String::from("ye've earned it!!")];
					gui.write_long_msg(&v, false);
				} else {
					break;
				}
			}
		}
	}

	let mut menu = Vec::new();
	let mut s = String::from("Tell us about yerself, ");
	s.push_str(&player_name);
	s.push(':');
	menu.push(s);
	menu.push("".to_string());
	menu.push("  (a) Are ye a fresh swab, full of vim and vigour. New to the".to_string());
	menu.push("      seas but ready to make a name for yerself? Ye'll be".to_string());
	menu.push("      quicker on yer toes but a tad wet behind yer ears.".to_string());
	menu.push("".to_string());
	menu.push("  (b) Or are ye an old sea dog? Ye've seen at least six of".to_string());
	menu.push("      the seven seas and yer hide is tougher for it. Yer peg".to_string());
	menu.push("      leg slows you down but experience has taught ye a few".to_string());
	menu.push("      tricks. And ye start with yer trusty flintlock.".to_string());

	let npcs: NPCTable = HashMap::new();

	let answer = gui.menu_picker(&menu, 2, true, true).unwrap();
	let state: GameState;
	if answer.contains(&0) {
	 	state = GameState::new_pirate(player_name, PirateType::Swab, npcs);
	} else {
	 	state = GameState::new_pirate(player_name, PirateType::Seadog, npcs);
	}
	
	state
}

fn death(state: &mut GameState, src: String, gui: &mut GameUI) {
	let sbi = state.curr_sidebar_info();
	state.write_msg_buff("Game over! --More--");
	gui.write_screen(&mut state.msg_buff, &sbi);
	gui.pause_for_more();
	
	let mut lines = vec![String::from("")];
	if src == "victory!" {
		lines.push(String::from("Well blow me down! Ye've found the lost treasure of"));
		let s = format!("{}! Yer fame, and fortune, are assured and pirates will be", state.pirate_lord);
		lines.push(s);
		lines.push(String::from("talling tales of your exploits for years to come!"));
		lines.push(String::from(""));
		let s = format!("Congratulations, Captain {}!", state.player.name);
		lines.push(s);
	} else if src != "quit" {
		let s = format!("Well shiver me timbers, {}, ye've died!", state.player.name);
		lines.push(s);
		lines.push(String::from(""));

		if src == "swimming" {
			lines.push(String::from("Ye died from drowning! Davy Jones'll have you for sure!"));
		} else if src == "venom" {
			lines.push(String::from("Ye died from venom!"));
		} else if src == "burn" {
			lines.push(String::from("Ye burned to death!"));
		} else if src == "falling" {
			lines.push(String::from("Ye took a nasty fall! But it's like they say: it don't be the fall"));
			lines.push(String::from("what gets you, it be the landing..."));
		} else  {
			let s = format!("Killed by a {}!", src);
			lines.push(s);
		}
	} else {
		lines.push(String::from("Ye've quit. Abandoned your quest and the treasure. Perhaps the next"));
		lines.push(String::from("pirate will have more pluck."));
	}

	if src != "victory!" {
		lines.push(String::from(""));
		let s = format!("{}'s treasure remains for some other swab...", state.pirate_lord);
		lines.push(s);
		lines.push(String::from(""));
	}
	
	let s = format!("So long, mate!");
	lines.push(s);

	gui.write_long_msg(&lines, true);
}

fn attack_player(state: &mut GameState, npc: &Monster) -> bool {
	do_ability_check(npc.hit_bonus, state.player.ac, 0)
}

fn prologue(state: &GameState, gui: &mut GameUI) {
	let mut lines = Vec::new();
	lines.push("Five days nigh you were looking for work in a seedy tavern near King's".to_string()); 
	lines.push("Quay when you overheard two old sailors talking about having got their".to_string()); 
	let s = format!("paws on a clue to the treasure of {}!", state.pirate_lord);
	lines.push(s);
	lines.push("".to_string());
	lines.push("The tales -- if ye can believe 'em -- have the pirate captain lost at".to_string());
	lines.push("sea in a storm, off the Yendorian Main. Many a sea dog has gone a'".to_string());
	lines.push("treasure hunting there and those who've retuned have come back with".to_string());
	lines.push("naught but talk of sharks, merfolk, the undead and still more dangers.".to_string());
	let s = format!("The stories say only one who has Captain {}'s eye", state.pirate_lord);
	lines.push(s);
	lines.push("patch, enchanted by a sea witch, will be able to find his hoard.".to_string());
	lines.push("".to_string());

	if state.starter_clue == 0 {
		lines.push("The sailors talked about searching the Obstreperous Strait and a map".to_string());
		lines.push("to one of the old pirates' caches. When they got too far into their".to_string());
		lines.push("cups, you saw your chance and pilfered the map.".to_string()); 
	} else {
		lines.push("The sailors had heard from a lobster fisherman who heard it from".to_string());
		lines.push("a priest that the pirate had been sailing the Obstreperous Strait".to_string());
		let s = format!("in the {} when a sudden, fierce squall sunk them. A clue", state.pirate_lord_ship);
		lines.push(s);
		lines.push("may found, if the wreck can be located.".to_string());
	}

	lines.push("".to_string());
	let s = format!("You spent the last of your coin on a keelboat, the {}", state.player_ship);
	lines.push(s);
	lines.push("and set out to the Obstreperous Straight. Having arrived, it's".to_string());
	lines.push("time to find a lost treasure and earn a place in tavern tales".to_string());
	lines.push("and sea shanties!".to_string());

	gui.write_long_msg(&lines, true);
}

fn start_game() {
    let ttf_context = sdl2::ttf::init()
		.expect("Error creating ttf context on start-up!");
	let font_path: &Path = Path::new("DejaVuSansMono.ttf");
    let font = ttf_context.load_font(font_path, 24)
		.expect("Error loading game font!");
	let sm_font = ttf_context.load_font(font_path, 18)
		.expect("Error loading small game font!");
	let mut gui = GameUI::init(&font, &sm_font)
		.expect("Error initializing GameUI object.");

	title_screen(&mut gui);

	let mut ships: HashMap<(usize, usize), Ship> = HashMap::new();
	let mut state = preamble(&mut gui, &mut ships);
	show_character_sheet(&state, &mut gui);
	let mut items = ItemsTable::new();

	generate_world(&mut state, &mut items, &mut ships);
	prologue(&state, &mut gui);

	match run(&mut gui, &mut state, &mut items, &mut ships) {
		Ok(_) => println!("Game over I guess? Probably the player won?!"),
		Err(src) => death(&mut state, src, &mut gui),
	}
}

fn run(gui: &mut GameUI, state: &mut GameState, 
		items: &mut ItemsTable, ships: &mut HashMap<(usize, usize), Ship>) -> Result<(), String> {

	state.write_msg_buff(&format!("Welcome, {}!", state.player.name));
	gui.v_matrix = fov::calc_v_matrix(state, items, ships, FOV_HEIGHT, FOV_WIDTH);
	let sbi = state.curr_sidebar_info();
	gui.write_screen(&mut state.msg_buff, &sbi);
	state.msg_buff.drain(..0);

    loop {
		let start_turn = state.turn;
		if state.player.charmed {
			action_while_charmed(state, items, ships)?;
		} else {
			let cmd = gui.get_command(&state);
			match cmd {
				Cmd::Quit => confirm_quit(state, gui)?,
				Cmd::Move(dir) => do_move(state, items, ships, &dir)?,
				Cmd::MsgHistory => show_message_history(state, gui),
				Cmd::DropItem => drop_item(state, items, gui),
				Cmd::PickUp => pick_up(state, items, gui)?,
				Cmd::ShowInventory => show_inventory(state, gui),
				Cmd::ShowCharacterSheet => show_character_sheet(state, gui),
				Cmd::ToggleEquipment => toggle_equipment(state, gui),
				Cmd::ToggleAnchor => {
					if toggle_anchor(state, ships) {
						sail(state, ships)?;
					}
				}
				Cmd::Pass => {
					if state.player.on_ship {
						sail(state, ships)?;
					}
					state.turn += 1;
				},
				Cmd::TurnWheelClockwise => {
					turn_wheel(state, ships, 1);
					sail(state, ships)?;
				},
				 Cmd::TurnWheelAnticlockwise => {
					turn_wheel(state, ships, -1);
					sail(state, ships)?;
				},
				Cmd::ToggleHelm => {
					if !state.player.on_ship {
						take_helm(state, ships);
					} else {
						leave_helm(state);
					}
				},
				Cmd::Quaff => quaff(state, gui),
				Cmd::Eat => eat(state, gui),
				Cmd::FireGun => fire_gun(state, gui, items, ships),
				Cmd::Reload => reload(state, gui),
				Cmd::WorldMap => gui.show_world_map(state),
				Cmd::Search => search(state, items),
				Cmd::Read => read(state, gui),
			}
		}

		// Some of the commands don't count as a turn for the player, so
		// don't give the monsters a free move in those cases
		if state.turn > start_turn {
			check_environment_hazards(state)?;

			let locs = state.npcs.keys()
						.map(|v| v.clone())
						.collect::<Vec<(usize, usize)>>();

			for loc in locs {
				let d = util::cartesian_d(loc.0, state.player.row, loc.1, state.player.col);
				if d < 75 { 
					let mut npc = state.npcs.remove(&loc).unwrap();
					npc.act(state, ships)?;
					let npc_r = npc.row;
					let npc_c = npc.col;
					state.npcs.insert((npc.row, npc.col), npc);
				}
			}

			if state.player.poisoned {
				let con_mod = Player::mod_for_stat(state.player.constitution);
				if do_ability_check(con_mod, 13, 0) {
					state.write_msg_buff("You feel better.");
					state.player.poisoned = false;
				} else {
					player_takes_dmg(&mut state.player, 1, "venom")?;
				}
			}

			if state.player.charmed {
				let verve_mod = Player::mod_for_stat(state.player.verve);
				let bonus = f32::round(state.player.drunkeness as f32 / 5.0) as i8;
				if do_ability_check(verve_mod, 14, bonus) {
					state.write_msg_buff("You snap out of it!");
					state.player.charmed = false;
				}
			}

			if state.player.drunkeness > 0 {
				state.player.drunkeness -= 1;
			}

			if state.turn % 25 == 0 {
				state.player.add_stamina(1);
			}
		}
	
		gui.v_matrix = fov::calc_v_matrix(state, items, ships, FOV_HEIGHT, FOV_WIDTH);
		let sbi = state.curr_sidebar_info();
		gui.write_screen(&mut state.msg_buff, &sbi);
		
		state.msg_buff.drain(..);	
    }

	Ok(())
}

fn main() {
	start_game();
}

