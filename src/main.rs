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
mod dice;
mod display;
mod fov;
mod items;
#[allow(dead_code)]
mod map;
#[allow(dead_code)]
mod pathfinding;
mod ship;

use crate::actor::{Monster, Player, PirateType};
use crate::dice::roll;
use crate::display::{GameUI, SidebarInfo};
use crate::items::{Item, ItemsTable};
use crate::ship::Ship;

use rand::Rng;

use std::collections::{HashMap, VecDeque};
use std::path::Path;

const MSG_HISTORY_LENGTH: usize = 50;
const FOV_WIDTH: usize = 41;
const FOV_HEIGHT: usize = 21;

pub type Map = Vec<Vec<map::Tile>>;
type NPCTable = HashMap<(usize, usize), Monster>;

pub enum Cmd {
	Exit,
	MoveN,
	MoveS,
	MoveE,
	MoveW,
	MoveNW,
	MoveNE,
	MoveSW,
	MoveSE,
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
}

pub struct GameState {
	player: Player,
	msg_buff: VecDeque<String>,
	msg_history: VecDeque<(String, u32)>,
	turn: u32,
}

impl GameState {
	pub fn new_pirate(name: String, p_type: PirateType) -> GameState {
		let player = match p_type {
			PirateType::Swab => Player::new_swab(name),
			PirateType::Seadog => Player::new_seadog(name),
		};

		GameState {player, msg_buff: VecDeque::new(), 
			msg_history: VecDeque::new(), turn: 0 }
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
				self.player.curr_stamina, self.player.max_stamina, wheel, bearing, self.turn)
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


fn get_move_tuple(mv: &str) -> (i16, i16) {
	let res: (i16, i16);

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

fn do_move(map: &Map, state: &mut GameState, npcs: &NPCTable, 
		items: &ItemsTable, ships: &HashMap<(usize, usize), Ship>, dir: &str) -> Result<(), String> {
	let mv = get_move_tuple(dir);

	let start_loc = (state.player.row, state.player.col);
	let start_tile = map[state.player.row][state.player.col];
	let next_row = state.player.row as i16 + mv.0;
	let next_col = state.player.col as i16 + mv.1;
	let next_loc = (next_row as usize, next_col as usize);
	let tile = map[next_row as usize][next_col as usize];
	
	if npcs.contains_key(&next_loc) {
		state.write_msg_buff("There is someone in your way!");
	}
	else if ships.contains_key(&next_loc) {
		state.player.col = next_col as usize;
		state.player.row = next_row as usize;
		let ship = ships.get(&next_loc).unwrap();
		let s = format!("You climb aboard the {}.", ship.name);
		state.write_msg_buff(&s);
		state.turn += 1;
	}
	else if map::is_passable(tile) {
		state.player.col = next_col as usize;
		state.player.row = next_row as usize;

		if tile == map::Tile::Water {
			state.write_msg_buff("You splash in the shallow water.");
		} else if tile == map::Tile::DeepWater {
			if start_tile != map::Tile::DeepWater {
				state.write_msg_buff("You begin to swim.");
			}

			player_takes_dmg(&mut state.player, 2, "swimming")?;

			if state.player.curr_stamina < 10 {
				state.write_msg_buff("You're getting tired...");
			}
		} else if start_tile == map::Tile::DeepWater && state.player.curr_stamina < 10 {
			state.write_msg_buff("Whew, you stumble ashore.");
		}

		let items_count = items.count_at(state.player.row, state.player.col);
		if items_count == 1 {
			let i = items.peek_top(state.player.row, state.player.col);
			let s = format!("You see a {} here.", i.name);
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
fn pluralize(name: &str, count: u8) -> String{
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
							let pluralized = pluralize(&pile[0].name, v);
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
				let s = format!("You drop the {}.", item.name);
				items.add(state.player.row, state.player.col, item);	
				state.write_msg_buff(&s);
				state.turn += 1;
			}	
		},
		None => state.write_msg_buff("Nevermind."),
	}

	state.player.calc_ac();
}

fn pick_up(state: &mut GameState, items: &mut ItemsTable, gui: &mut GameUI) {
	let item_count = items.count_at(state.player.row, state.player.col);
	if item_count == 0 {
		state.write_msg_buff("There is nothing here to pick up.");
	} else if item_count == 1 {
		let item = items.get_at(state.player.row, state.player.col);
		let s = format!("You pick up the {}.", item.name);
		state.player.inventory.add(item);
		state.write_msg_buff(&s);
		state.turn += 1;
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
					let s = format!("You pick up the {}.", item.name);
					state.player.inventory.add(item);
					state.write_msg_buff(&s);
				}
			},
		}
	}
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
fn sq_open(map: &Map, state: &GameState, ships: &HashMap<(usize, usize), Ship>, row: usize, col: usize) -> bool {
	if !map::in_bounds(map, row as i32, col as i32) {
		return false;
	}

	if !map::is_passable(map[row][col]) {
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

fn get_open_sq_adj_player(map: &Map, state: &GameState, ships: &HashMap<(usize, usize), Ship>) -> Option<(usize, usize)> {
	let mut sqs: Vec<(usize, usize)> = Vec::new();
	if sq_open(map, state, ships, state.player.row - 1, state.player.col - 1) {
		sqs.push((state.player.row - 1, state.player.col - 1));
	}
	if sq_open(map, state, ships, state.player.row - 1, state.player.col) {
		sqs.push((state.player.row - 1, state.player.col));
	}
	if sq_open(map, state, ships, state.player.row - 1, state.player.col + 1) {
		sqs.push((state.player.row - 1, state.player.col + 1));
	}
	if sq_open(map, state, ships, state.player.row, state.player.col + 1) {
		sqs.push((state.player.row, state.player.col + 1));
	}
	if sq_open(map, state, ships, state.player.row, state.player.col - 1) {
		sqs.push((state.player.row, state.player.col - 1));
	}
	if sq_open(map, state, ships, state.player.row + 1, state.player.col - 1) {
		sqs.push((state.player.row + 1, state.player.col - 1));
	}
	if sq_open(map, state, ships, state.player.row + 1, state.player.col) {
		sqs.push((state.player.row + 1, state.player.col));
	}
	if sq_open(map, state, ships, state.player.row + 1, state.player.col + 1) {
		sqs.push((state.player.row + 1, state.player.col + 1));
	}

	if sqs.len() == 0 {
		None
	} else {
		let j = (dice::roll(sqs.len() as u8 + 1, 1, 0) - 1) as usize;
		let loc = sqs[j];
		Some(loc)
	}
}

fn ship_hit_land(map: &Map, state: &mut GameState, ship: &mut Ship, ships: &HashMap<(usize, usize), Ship>) -> Result<(), String> {
	state.write_msg_buff("Ye've run yer ship aground!!");
	state.write_msg_buff("You lose control o' the wheel!");
	let mut new_wheel = ship.wheel + 2 + dice::roll(5, 1, 0) as i8;	
	new_wheel = new_wheel % 5 - 2;
	ship.wheel = new_wheel;
	state.player.wheel = new_wheel;

	if !do_ability_check(Player::mod_for_stat(state.player.dexterity), 13, 0) {
		if let Some(loc)= get_open_sq_adj_player(map, state, ships) {
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

fn sail(map: &Map, state: &mut GameState, ships: &mut HashMap<(usize, usize), Ship>) -> Result<(), String> {
	let mut ship = ships.remove(&(state.player.row, state.player.col)).unwrap();

	let bow_tile = map[ship.bow_row][ship.bow_col];
	let ship_tile = map[ship.row][ship.col];

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

		state.player.row = (state.player.row as i8 + delta.0) as usize;
		state.player.col = (state.player.col as i8 + delta.1) as usize;
		ship.row = (ship.row as i8 + delta.0) as usize;
		ship.col = (ship.col as i8 + delta.1) as usize;
		ship.update_loc_info();
		ship.prev_move = delta;

		//if map[ship.row][ship.col] == map::Tile::Water || 
		if map[ship.bow_row][ship.bow_col] == map::Tile::Water {
			state.write_msg_buff("Shallow water...");
		} else if map[ship.bow_row][ship.bow_col] != map::Tile::DeepWater {
			ship_hit_land(map, state, &mut ship, ships)?;
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

fn show_title_screen(gui: &mut GameUI) {
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

fn add_monster(map: &Map, state: &mut GameState, npcs: &mut NPCTable) {
	let mut row = 0;
	let mut col = 0;
	loop {
		row = rand::thread_rng().gen_range(0, map.len());
		col = rand::thread_rng().gen_range(0, map[0].len());

		let tile = map[row][col];
		if tile == map::Tile::DeepWater { break; }
	}	
	
	let s = actor::Monster::new_shark(row, col);
	npcs.insert((row, col), s);
}

fn is_putting_on_airs(name: &str) -> bool {
	name.to_lowercase().starts_with("capt") ||
		name.to_lowercase().starts_with("capn") ||
		name.to_lowercase().starts_with("cap'n") 
}

fn preamble(map: &Map, gui: &mut GameUI, ships: &mut HashMap<(usize, usize), Ship>) -> GameState {
	let mut player_name: String;

	let sbi = SidebarInfo::new("".to_string(), 0, 0, 0, -1, -1, 0);
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

	let answer = gui.menu_picker(&menu, 2, true, true).unwrap();
	let mut state: GameState;
	if answer.contains(&0) {
	 	state = GameState::new_pirate(player_name, PirateType::Swab);
	} else {
	 	state = GameState::new_pirate(player_name, PirateType::Seadog);
	}
	state.player.on_ship = true;
	state.player.bearing = 0;
	state.player.wheel = 0;

	// Find a random starting place for a ship
	loop {
		let r = rand::thread_rng().gen_range(1, map.len() - 1);
		let c = rand::thread_rng().gen_range(1, map.len() - 1);
		if map[r][c] == map::Tile::DeepWater && map[r-1][c] == map::Tile::DeepWater 
				&& map[r+1][c]==  map::Tile::DeepWater {
			state.player.row = r;
			state.player.col = c;
			break;
		}
	}

	let mut ship = Ship::new("The Minnow".to_string());
	ship.row = state.player.row;
	ship.col = state.player.col;
	ship.bearing = 0;
	ship.wheel = 0;
	ship.update_loc_info();
	ships.insert((state.player.row, state.player.col), ship);

	state
}

fn death(state: &GameState, src: String, gui: &mut GameUI) {
	let mut lines = vec![String::from("")];
	let s = format!("Well shiver me timbers, {}, ye've died!", state.player.name);
	lines.push(s);

	if src == "swimming" {
		lines.push(String::from(""));
		lines.push(String::from("Ye died from drowning! Davy Jones'll have you for sure!"));
	} else if src == "falling" {
		lines.push(String::from(""));
		lines.push(String::from("Ye took a nasty fall! But it's like they say: it don't be the fall"));
		lines.push(String::from("what gets you, it be the landing..."));
	} else  {
		let s = format!("Killed by a {}", src);
	}

	lines.push(String::from(""));
	lines.push(String::from("Silverbeard's treasure remains for some other swab..."));
	lines.push(String::from(""));

	gui.write_long_msg(&lines, true);
}

fn attack_player(state: &mut GameState, npc: &Monster) -> bool {
	do_ability_check(npc.hit_bonus, state.player.ac, 0)
}

fn npc_turns(map: &Map, state: &mut GameState, npcs: &mut NPCTable, items: &mut ItemsTable) -> Result<(), String> {
	let locs = npcs.keys()
					.map(|v| v.clone())
					.collect::<Vec<(usize, usize)>>();

	for loc in locs {
		let mut npc = npcs.remove(&loc).unwrap();
		npc.act(state, map, npcs)?;
		npcs.insert((npc.row, npc.col), npc);
	}

	Ok(())	
}

fn start_game(map: &Map) {
    let ttf_context = sdl2::ttf::init()
		.expect("Error creating ttf context on start-up!");
	let font_path: &Path = Path::new("DejaVuSansMono.ttf");
    let font = ttf_context.load_font(font_path, 24)
		.expect("Error loading game font!");
	let sm_font = ttf_context.load_font(font_path, 18)
		.expect("Error loading small game font!");
	let mut gui = GameUI::init(&font, &sm_font)
		.expect("Error initializing GameUI object.");

	show_title_screen(&mut gui);

	let mut ships: HashMap<(usize, usize), Ship> = HashMap::new();
	let mut state = preamble(&map, &mut gui, &mut ships);

	show_character_sheet(&state, &mut gui);
	
	let mut npcs: NPCTable = HashMap::new();

	let mut items = ItemsTable::new();

	match run(&mut gui, &mut state, &map, &mut npcs, &mut items, &mut ships) {
		Ok(_) => println!("Game over I guess? Probably the player won?!"),
		Err(src) => death(&state, src, &mut gui),
	}
}

fn run(gui: &mut GameUI, state: &mut GameState, map: &Map,
		npcs: &mut NPCTable, items: &mut ItemsTable, ships: &mut HashMap<(usize, usize), Ship>) -> Result<(), String> {
	add_monster(map, state, npcs);
	add_monster(map, state, npcs);
	add_monster(map, state, npcs);

	state.write_msg_buff(&format!("Welcome, {}!", state.player.name));
	gui.v_matrix = fov::calc_v_matrix(map, npcs, items, ships, &state.player, FOV_HEIGHT, FOV_WIDTH);
	let sbi = state.curr_sidebar_info();
	gui.write_screen(&mut state.msg_buff, &sbi);

    'mainloop: loop {
		let mut update = false;
		let cmd = gui.get_command(&state);
		match cmd {
			Cmd::Exit => break 'mainloop,
			Cmd::MoveW => {
				do_move(map, state, npcs, items, ships, "W")?;
				update = true;
			},
			Cmd::MoveS => {
				do_move(map, state, npcs, items, ships, "S")?;
				update = true;
			},
			Cmd::MoveN => {
				do_move(map, state, npcs, items, ships, "N")?;
				update = true;
			},
			Cmd::MoveE => {
				do_move(map, state, npcs, items, ships, "E")?;
				update = true;
			},
			Cmd::MoveNW => {
				do_move(map, state, npcs, items, ships, "NW")?;
				update = true;
			},
			Cmd::MoveNE => {
				do_move(map, state, npcs, items, ships, "NE")?;
				update = true;
			},
			Cmd::MoveSW => {
				do_move(map, state, npcs, items, ships, "SW")?;
				update = true;
			},
			Cmd::MoveSE => {
				do_move(map, state, npcs, items, ships, "SE")?;
				update = true;
			},
			Cmd::MsgHistory => {
				show_message_history(state, gui);
				update = true;
			},
			Cmd::DropItem => {
				drop_item(state, items, gui);
				update = true;
			}
			Cmd::PickUp => {
				pick_up(state, items, gui);
				update = true;
			}
			Cmd::ShowInventory => {
				show_inventory(state, gui);
				update = true;
			},
			Cmd::ShowCharacterSheet => {
				show_character_sheet(state, gui);
				update = true;
			},
			Cmd::ToggleEquipment => {
				toggle_equipment(state, gui);
				update = true;
			},
			Cmd::ToggleAnchor => {
				if toggle_anchor(state, ships) {
					sail(map, state, ships)?;
				}
				update = true;
			}
			Cmd::Pass => {
				if state.player.on_ship {
					sail(map, state, ships)?;
					update = true;
					state.turn += 1
				}
			},
			Cmd::TurnWheelClockwise => {
				turn_wheel(state, ships, 1);
				sail(map, state, ships)?;
				update = true;
			},
			 Cmd::TurnWheelAnticlockwise => {
				turn_wheel(state, ships, -1);
				sail(map, state, ships)?;
				update = true;
			},
			Cmd::ToggleHelm => {
				if !state.player.on_ship {
					take_helm(state, ships);
				} else {
					leave_helm(state);
				}
				update = true;
			},
        }
	
		if update {
			gui.v_matrix = fov::calc_v_matrix(map, npcs, items, ships, &state.player, FOV_HEIGHT, FOV_WIDTH);
			let sbi = state.curr_sidebar_info();
			gui.write_screen(&mut state.msg_buff, &sbi);
		}

		npc_turns(map, state, npcs, items);
    }

	Ok(())
}

fn main() {
	let map = map::generate_island(65);
	//let map = map::generate_cave(20, 10);
	//let path = pathfinding::find_path(&map, 4, 4, 9, 9);
	
	start_game(&map);
}

