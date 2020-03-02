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

use crate::actor::{Act, Player, PirateType};
use crate::dice::roll;
use crate::display::GameUI;
use crate::items::{Item, ItemsTable};

use rand::Rng;

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::rc::Rc;

const MSG_HISTORY_LENGTH: usize = 50;
const FOV_WIDTH: usize = 41;
const FOV_HEIGHT: usize = 21;

pub type Map = Vec<Vec<map::Tile>>;
type NPCTable = HashMap<(usize, usize), Rc<RefCell<dyn actor::Act>>>;

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
}

pub struct GameState {
	player: Player,
	msg_buff: VecDeque<String>,
	msg_history: VecDeque<(String, u32)>,
}

impl GameState {
	pub fn new_pirate(name: String, p_type: PirateType) -> GameState {
		let player = match p_type {
			PirateType::Swab => Player::new_swab(name),
			PirateType::Seadog => Player::new_seadog(name),
		};

		GameState {player, msg_buff: VecDeque::new(),
			msg_history: VecDeque::new() }
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

fn do_move(map: &Map, state: &mut GameState, npcs: &NPCTable, items: &ItemsTable, dir: &str) {
	let mv = get_move_tuple(dir);
	let next_row = state.player.row as i16 + mv.0;
	let next_col = state.player.col as i16 + mv.1;
	let tile = map[next_row as usize][next_col as usize];
	
	if npcs.contains_key(&(next_row as usize, next_col as usize)) {
		state.write_msg_buff("There is someone in your way!");
	}
	else if map::is_passable(tile) {
		state.player.col = next_col as usize;
		state.player.row = next_row as usize;

		if tile == map::Tile::Water {
			state.write_msg_buff("You splash in the shallow water.");
		} 

		let items_count = items.count_at(state.player.row, state.player.col);
		if items_count == 1 {
			let i = items.peek_top(state.player.row, state.player.col);
			let s = format!("You see a {} here.", i.name);
			state.write_msg_buff(&s);
		} else if items_count > 1 {
			state.write_msg_buff("You see a few items here.");
		}	
	} else  {
		if tile == map::Tile::DeepWater {
			state.write_msg_buff("You cannot swim!");
		} else {
			state.write_msg_buff("You cannot go that way.");
		}
	}
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

	match gui.query_single_response("Drop what?") {
		Some(ch) =>  {
			let count = state.player.inventory.count_in_slot(ch);
			if count == 0 {
				state.write_msg_buff("You do not have that item.");
			} else if count > 1 {
				match gui.query_natural_num("Drop how many?") {
					Some(v) => {
						let pile = state.player.inventory.remove_count(ch, v);
						if pile.len() > 0 {
							let pluralized = pluralize(&pile[0].name, v);
							let s = format!("You drop {} {}", v, pluralized);
							state.write_msg_buff(&s);
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
			}	
		},
		None => state.write_msg_buff("Nevermind."),
	}
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
	} else {
		let mut menu = items.get_menu(state.player.row, state.player.col);
		menu.insert(0, "Pick up what: (* to get everything)".to_string());
		let answers = gui.menu_picker(&menu, menu.len() as u8, false, false);
		match answers {
			None => state.write_msg_buff("Nevermind."), // Esc was pressed
			Some(v) => {
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
	let s = format!("AC: {}    Stamina: {}", state.player.ac, state.player.stamina);
	lines.push(s);

	gui.write_long_msg(&lines, true);
}

fn show_intro(gui: &mut GameUI) {
	let mut lines = vec!["Welcome to YarrL, a roguelike adventure on the high seas!".to_string(), "".to_string()];
	lines.push("".to_string());
	lines.push("".to_string());
	lines.push("".to_string());
	lines.push("Copyright 2020 by Dana Larose, see COPYING for licence info.".to_string());
	
	gui.write_long_msg(&lines, true);
}

fn add_monster(map: &Map, state: &mut GameState, npcs: &mut NPCTable) {
	let mut row = 0;
	let mut col = 0;
	loop {
		row = rand::thread_rng().gen_range(0, map.len());
		col = rand::thread_rng().gen_range(0, map[0].len());

		let tile = map[row][col];
		if map::is_passable(tile) { break; };
	}	
	
	let mut m = actor::Monster::new(13, 25, 'o', row, col, display::BLUE);
	npcs.insert((row, col), Rc::new(RefCell::new(m)));
}

fn is_putting_on_airs(name: &str) -> bool {
	name.to_lowercase().starts_with("capt") ||
		name.to_lowercase().starts_with("capn") ||
		name.to_lowercase().starts_with("cap'n") 
}

fn preamble(map: &Map, gui: &mut GameUI) -> GameState {
	let mut player_name: String;
	loop {
		if let Some(name) = gui.query_user("Ahoy lubber, who be ye?", 15) {
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

	loop {
		let r = rand::thread_rng().gen_range(1, map.len() - 1);
		let c = rand::thread_rng().gen_range(1, map.len() - 1);
		match map[r][c] {
			map::Tile::Water | map::Tile::Wall | map::Tile::DeepWater |
			map::Tile::Mountain | map::Tile::SnowPeak => { continue; },
			_ => {
				state.player.row = r;
				state.player.col = c;
				break;
			}
		}
	}

	state
}

fn run(map: &Map) {
    let ttf_context = sdl2::ttf::init()
		.expect("Error creating ttf context on start-up!");
	let font_path: &Path = Path::new("DejaVuSansMono.ttf");
    let font = ttf_context.load_font(font_path, 24)
		.expect("Error loading game font!");
	let sm_font = ttf_context.load_font(font_path, 18)
		.expect("Error loading small game font!");
	let mut gui = GameUI::init(&font, &sm_font)
		.expect("Error initializing GameUI object.");

	show_intro(&mut gui);

	let mut state = preamble(&map, &mut gui);

	show_character_sheet(&state, &mut gui);
	
	let mut npcs: NPCTable = HashMap::new();
	add_monster(map, &mut state, &mut npcs);

	let mut items = ItemsTable::new();

	state.write_msg_buff(&format!("Welcome, {}!", state.player.name));
	gui.v_matrix = fov::calc_v_matrix(&map, &npcs, &items,
		state.player.row, state.player.col, FOV_HEIGHT, FOV_WIDTH);
	gui.write_screen(&mut state.msg_buff);
	

    'mainloop: loop {
		//let mut m = npcs.get(&(17, 17)).unwrap().borrow_mut();
		//let initiative_order = vec![m];

		let mut update = false;
		let cmd = gui.get_command();
		match cmd {
			Cmd::Exit => break 'mainloop,
			Cmd::MoveW => {
				do_move(&map, &mut state, &npcs, &items, "W");
				update = true;
			},
			Cmd::MoveS => {
				do_move(&map, &mut state, &npcs, &items, "S");
				update = true;
			},
			Cmd::MoveN => {
				do_move(&map, &mut state, &npcs, &items, "N");
				update = true;
			},
			Cmd::MoveE => {
				do_move(&map, &mut state, &npcs, &items, "E");
				update = true;
			},
			Cmd::MoveNW => {
				do_move(&map, &mut state, &npcs, &items, "NW");
				update = true;
			},
			Cmd::MoveNE => {
				do_move(&map, &mut state, &npcs, &items, "NE");
				update = true;
			},
			Cmd::MoveSW => {
				do_move(&map, &mut state, &npcs, &items, "SW");
				update = true;
			},
			Cmd::MoveSE => {
				do_move(&map, &mut state, &npcs, &items, "SE");
				update = true;
			},
			Cmd::MsgHistory => {
				show_message_history(&state, &mut gui);
				update = true;
			},
			Cmd::DropItem => {
				drop_item(&mut state, &mut items, &mut gui);
				update = true;
			}
			Cmd::PickUp => {
				pick_up(&mut state, &mut items, &mut gui);
				update = true;
			}
			Cmd::ShowInventory => {
				show_inventory(&mut state, &mut gui);
				update = true;
			},
			Cmd::ShowCharacterSheet => {
				show_character_sheet(&state, &mut gui);
				update = true;
			},
        }
	
		if update {
			gui.v_matrix = fov::calc_v_matrix(&map, &npcs, &items,
				state.player.row, state.player.col, FOV_HEIGHT, FOV_WIDTH);
			gui.write_screen(&mut state.msg_buff);
		}
    }
}

fn main() {
	let map = map::generate_island(65);
	//let map = map::generate_cave(20, 10);
	//let path = pathfinding::find_path(&map, 4, 4, 9, 9);
	
	run(&map);
}

