extern crate rand;
extern crate sdl2;

mod actor;
mod display;
mod fov;
mod items;
#[allow(dead_code)]
mod map;
#[allow(dead_code)]
mod pathfinding;

use crate::actor::{Act, Player};
use crate::display::GameUI;
use crate::items::ItemsTable;

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

enum Cmd {
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
}

pub struct GameState {
	player: Player,
	msg_buff: VecDeque<String>,
	msg_history: VecDeque<(String, u32)>,
}

impl GameState {
	pub fn new(name: String) -> GameState {
		let mut player = Player::new(name);

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
							for item in pile {
								items.add(state.player.row, state.player.col, item);
							}
						} else {
							state.write_msg_buff("Nevermind.");
						}
					},
					None => state.write_msg_buff("Nevermind."),
				}
			} else {
				let item = state.player.inventory.remove(ch);
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
		let answers = gui.menu_picker(&menu, menu.len() as u8);
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

fn show_intro(gui: &mut GameUI) {
	let mut lines = vec!["Welcome to a rogulike UI prototype!".to_string(), "".to_string()];
	lines.push("You can move around with vi-style keys and bump".to_string());
	lines.push("into water and mountains.".to_string());
	lines.push("".to_string());
	lines.push("There are no monsters or anything yet, though!".to_string());
	
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

fn add_test_item(map: &Map, items: &mut ItemsTable) {
	let mut row = 0;
	let mut col = 0;
	loop {
		row = rand::thread_rng().gen_range(0, map.len());
		col = rand::thread_rng().gen_range(0, map[0].len());

		let tile = map[row][col];
		if map::is_passable(tile) { break; };
	}	

	let i = items::Item::new("draught of rum", items::ItemType::Drink, 1, true,
		'!', display::BROWN);
	items.add(row, col, i);	

	let i = items::Item::new("rusty cutlass", items::ItemType::Weapon, 3, false,
		'|', display::WHITE);
	items.add(row, col, i);	

	let i = items::Item::new("draught of rum", items::ItemType::Drink, 1, true,
		'!', display::BROWN);
	items.add(row, col + 1, i);	

	let i = items::Item::new("draught of rum", items::ItemType::Drink, 1, true,
		'!', display::BROWN);
	items.add(row + 1, col, i);	

	let i = items::Item::new("draught of gin", items::ItemType::Drink, 1, true,
		'!', display::WHITE);
	items.add(row - 1, col, i);	
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
		
	let player_name = gui.query_user("Who are you?");
	let mut state = GameState::new(player_name);
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
	
	let mut npcs: NPCTable = HashMap::new();
	add_monster(map, &mut state, &mut npcs);

	let mut items = ItemsTable::new();
	add_test_item(map, &mut items);

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
			}
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
