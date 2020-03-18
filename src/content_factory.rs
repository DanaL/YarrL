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

use std::collections::{HashMap, HashSet, VecDeque};

use rand::Rng;

use super::{GameState, ItemsTable, ShipsTable};
use crate::actor::NPCTracker;
use crate::dice;
use crate::items::Item;
use crate::map;
use crate::map::Tile;
use crate::ship;
use crate::ship::Ship;
use crate::util;
use crate::util::rnd_adj;
use crate::weather::{Weather, WeatherSystem};

pub const WORLD_WIDTH: usize = 250;
pub const WORLD_HEIGHT: usize = 250;

struct IslandInfo {
	coastline: VecDeque<(usize, usize)>,
	length: usize,
	offset_r: usize,
	offset_c: usize,
}

impl IslandInfo {
	fn new(offset_r: usize, offset_c: usize) -> IslandInfo {
		IslandInfo { coastline: VecDeque::new(), length: 0, offset_r, offset_c }
	}
}

fn initialize_map(state: &mut GameState) {
	let mut top = Vec::new();
	for _ in 0..WORLD_WIDTH {
		top.push(Tile::WorldEdge);
	}
	let curr_map = state.map.get_mut(&0).unwrap();
	curr_map.push(top);

	for _ in 0..WORLD_HEIGHT - 2 {
		let mut row = Vec::new();
		row.push(Tile::WorldEdge);
		for _ in 0..WORLD_WIDTH - 2 {
			row.push(Tile::DeepWater);
		}
		row.push(Tile::WorldEdge);
		curr_map.push(row);
	}

	let mut bottom = Vec::new();
	for _ in 0..WORLD_WIDTH {
		bottom.push(Tile::WorldEdge);
	}
	curr_map.push(bottom);
}

pub fn generate_world(state: &mut GameState,
		items: &mut HashMap<u8, ItemsTable>,
		ships: &mut HashMap<u8, ShipsTable>) {

	initialize_map(state);
	// at the moment I have two clue types: maps and 
	// shipwrecks.
	//
	// One I have implenented caves and hidden valleys
	// then clues can be hidden in them as well
	let clue_1 = if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		0
	} else {
		1
	};
 
	let clue_2 = if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		0
	} else {
		1
	};

	let final_clue = if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		0
	} else {
		1
	};

	let mut q1_info = IslandInfo::new(5, 5);
	create_island(state, items, &mut q1_info, ships);
	let mut q2_info = IslandInfo::new(10, 100);
	create_island(state, items, &mut q2_info, ships);
	let mut q3_info = IslandInfo::new(100, 10);
	create_island(state, items, &mut q3_info, ships);
	let mut q4_info = IslandInfo::new(100, 100);
	create_island(state, items, &mut q4_info, ships);
	let islands = vec![q1_info, q2_info, q3_info, q4_info];

	state.pirate_lord = get_pirate_lord();
	// the player isn't allowed to have a royal Yendorian naval ship
	state.player_ship = ship::random_name(false);
	state.starter_clue = clue_1;

	// the final mcguffin is always found by a treasure map, to keep the
	// player from just searching every shipwreck...
	let mut c = Vec::new();
	let chest = Item::get_macguffin(&state.pirate_lord);
 	c.push(chest);
	let roll = rand::thread_rng().gen_range(0, 4);
	let mut map_to_chest = set_treasure_map(&state.map[&0], 
		&islands[roll], items.get_mut(&0).unwrap(), c, 0).unwrap();
	map_to_chest.hidden = true;

	let mut eye_patch = Item::get_item("magic eye patch").unwrap();
	eye_patch.hidden = true;
	let mut c = Vec::new();
	c.push(eye_patch);
	c.push(map_to_chest);
	let mut hint_to_final_clue;

	// We also need to include the location of the treasure along with the 
	// eye patch
	if final_clue == 0 {
		let roll = rand::thread_rng().gen_range(0, 4);
		hint_to_final_clue = set_treasure_map(&state.map[&0], 
			&islands[roll], items.get_mut(&0).unwrap(), c, 0).unwrap();
	} else {
		let roll = rand::thread_rng().gen_range(0, 4);
		let ship_name = add_shipwreck(state, &islands[roll], items.get_mut(&0).unwrap(), c, true);
		hint_to_final_clue = Item::get_note(state.note_count);
		state.notes.insert(state.note_count, Item::get_note_text(&ship_name));
		state.note_count += 1;
	}
	hint_to_final_clue.hidden = true;

	// Now the second clue
	let mut c = Vec::new();
	c.push(hint_to_final_clue);
	let mut hint_to_2nd_clue;

	if clue_2 == 0 {
		let roll = rand::thread_rng().gen_range(0, 4);
		hint_to_2nd_clue = set_treasure_map(&state.map[&0], 
			&islands[roll], items.get_mut(&0).unwrap(), c, 0).unwrap();
	} else {
		let roll = rand::thread_rng().gen_range(0, 4);
		let ship_name = add_shipwreck(state, &islands[roll], items.get_mut(&0).unwrap(), c, true);
		hint_to_2nd_clue = Item::get_note(state.note_count);
		state.notes.insert(state.note_count, Item::get_note_text(&ship_name));
		state.note_count += 1;
	}
	hint_to_2nd_clue.hidden = true;

	// Now the first clue
	let mut c = Vec::new();
	c.push(hint_to_2nd_clue);
	if clue_1 == 0 {
		let roll = rand::thread_rng().gen_range(0, 4);
		let map = set_treasure_map(&state.map[&0], &islands[roll], items.get_mut(&0).unwrap(), c, 0).unwrap();
		state.player.inventory.add(map);
	} else {
		let roll = rand::thread_rng().gen_range(0, 4);
		let ship_name = add_shipwreck(state, &islands[roll], items.get_mut(&0).unwrap(), c, true);
		state.pirate_lord_ship = ship_name.clone();
	}

	// place the player
	state.player.on_ship = true;
	state.player.bearing = 6;
	state.player.wheel = 0;
	state.player.row = 5;
	state.player.col = 5;

	let mut ship = Ship::new(state.player_ship.clone());
	ship.row = state.player.row;
	ship.col = state.player.col;
	ship.bearing = 6;
	ship.wheel = 0;
	ship.update_loc_info();
	ships.insert(0, HashMap::new());
	let curr_ships = ships.get_mut(&0).unwrap();
	curr_ships.insert((state.player.row, state.player.col), ship);

    let mut w = Weather::new();
    let ws = WeatherSystem::new(20, 20, 15, 0.5);
    w.systems.push(ws);
    let ws = WeatherSystem::new(45, 45, 20, 0.4);
    w.systems.push(ws);
    w.calc_clouds(state);
    state.weather.insert(0, w);
}

fn find_location_for_land_monster(world_map: &Vec<Vec<Tile>>, 
					info: &IslandInfo) -> (usize, usize) {
	loop {
		let r = rand::thread_rng().gen_range(info.offset_r, info.offset_r + info.length); 
		let c = rand::thread_rng().gen_range(info.offset_c, info.offset_c + info.length); 
		
		if world_map[r][c] == Tile::Grass || world_map[r][c] == Tile::Dirt ||
			world_map[r][c] == Tile::Tree || world_map[r][c] == Tile::Sand ||
			world_map[r][c] == Tile::Floor {
			return (r, c)
		}
	}
}

fn create_island(state: &mut GameState, 
					items: &mut HashMap<u8, ItemsTable>,
					island_info: &mut IslandInfo,
					ships: &mut HashMap<u8, ShipsTable>) {
	let island;
	let island_type = rand::thread_rng().gen_range(0.0, 1.0);
	let max_shipwrecks;
	let max_old_campsites;
	let max_campsites;
	let max_fruit;
	let mut spring = false;
	let mut skeleton_island = false;
    let mut has_cave = false;

	if island_type < 0.5 {
		// regular island
		island = map::generate_std_island();
		max_shipwrecks = 3;
		max_old_campsites = 4;
		max_fruit = 8;		
		max_campsites = 3;
		island_info.length = 65;
		spring = true;

		// Once in a while, an island will be occupied by an undead
		// skeleton captain who will raise an undead army 
		if rand::thread_rng().gen_range(0.0, 1.0) < 0.15 {
			skeleton_island = true;
		}

		if rand::thread_rng().gen_range(0.0, 1.0) < 1.00 {
            has_cave = true;
        }

	} else if island_type < 0.85 {
		// atoll
		island = map::generate_atoll();
		max_shipwrecks = 5;
		max_old_campsites = 3;
		max_fruit = 4; 
		max_campsites = 3;
		island_info.length = 129;
	} else {
		// volcano
		island = generate_volcanic_island();
		max_shipwrecks = 3;
		max_old_campsites = 3;
		max_fruit = 6; 
		max_campsites = 3;
		island_info.length = 65;
		spring = true;

		if rand::thread_rng().gen_range(0.0, 1.0) < 1.00 {
            has_cave = true;
        }
	}

	// this doesn't do what I wanted it to, I don't think
	// I want to transpose islands that are very small but this
	// just skips copying some of their water squares. But transposing
	// the island would break how I'm calculating the coastline anyhow
	// (or well I'm not taking into account the transposing that doesn't
	// work yet)
	let nw = find_nearest_clear_nw(&island);
	let curr_map = state.map.get_mut(&0).unwrap();
	for r in nw.0..island.len() {
		for c in nw.1..island.len() {
			curr_map[r + island_info.offset_r][c + island_info.offset_c] = island[r][c].clone();
		}
	}

	// find_hidden_valleys(&island);

	if spring && rand::thread_rng().gen_range(0.0, 1.0) < 0.33 {
		place_spring(state, island_info);
	}
	
	find_coastline(&state.map[&0], island_info);

	for _ in 0..rand::thread_rng().gen_range(0, max_shipwrecks) {
		let cache = get_cache_items();
		add_shipwreck(state, island_info, items.get_mut(&state.map_id).unwrap(), cache, false);
	}
	for _ in 0..rand::thread_rng().gen_range(0, max_old_campsites) {
		let curr_map = state.map.get_mut(&0).unwrap();
		set_old_campsite(curr_map, island_info, items.get_mut(&state.map_id).unwrap());
	}
	for _ in 0..rand::thread_rng().gen_range(0, max_fruit) {
		let curr_map = state.map.get(&0).unwrap();
		add_fruit(curr_map, island_info, items.get_mut(&state.map_id).unwrap());
	}

	if !skeleton_island {
		for _ in 0..rand::thread_rng().gen_range(0, max_campsites) {
			set_campsite(state, island_info, items.get_mut(&state.map_id).unwrap());
		}
	}

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.2 {
		let mut curr_map = state.map.get_mut(&0).unwrap();
		place_fort(&mut curr_map, island_info, items.get_mut(&state.map_id).unwrap());
	}

	if !skeleton_island {
		if rand::thread_rng().gen_range(0.0, 1.0) < 1.25 {
			set_castaway(state, island_info);
		}

		let npcs = state.npcs.get_mut(&0).unwrap();
		// let's add some monsters in 
		for _ in 2..rand::thread_rng().gen_range(3, 5) {
			let loc = find_location_for_land_monster(&state.map[&0], island_info);
            npcs.new_snake(loc.0, loc.1);
		}
		for _ in 1..rand::thread_rng().gen_range(2, 4) {
			let loc = find_location_for_land_monster(&state.map[&0], island_info);
            npcs.new_boar(loc.0, loc.1);
		}
		if rand::thread_rng().gen_range(0.0, 1.0) < 0.1 {
			let loc = find_location_for_land_monster(&state.map[&0], island_info);
            npcs.new_panther(loc.0, loc.1);
		}
	} else {
		let npcs = state.npcs.get_mut(&0).unwrap();
		let skellie_count = rand::thread_rng().gen_range(8, 11); 
		let loc = find_location_for_land_monster(&state.map[&0], island_info);
        let boss_id = npcs.new_undead_boss(loc.0, loc.1, skellie_count);
		for _ in 0..skellie_count {
			let loc = find_location_for_land_monster(&state.map[&0], island_info);
            npcs.new_skeleton(loc.0, loc.1, boss_id);
		}
	}

    if has_cave {
        place_cave(state, items, island_info, ships);
    }
}

fn get_pirate_lord() -> String {
	let ns = util::read_names_file();
	
	let j = rand::thread_rng().gen_range(0, ns.proper_nouns.len());

	ns.proper_nouns[j].clone()
}

fn pts_on_line(r: f32, c: f32, d: f32, angle: f32) -> (usize, usize) {
	let next_r = (r + (d * f32::sin(angle))) as usize;
	let next_c = (c + (d * f32::cos(angle))) as usize;

	(next_r, next_c)
}

fn draw_lava_flow(map: &mut Vec<Vec<Tile>>, start_r: usize, start_c: usize) {
	// I still think in degrees not radians...
	let mut angle = rand::thread_rng().gen_range(0.0, 360.0) * 0.01745329;
	let r = start_r as f32;
	let c = start_c as f32;
	let mut d = 0.0;
	
	loop {	
		let (next_r, next_c) = pts_on_line(r, c, d, angle); 
		if !map::in_bounds(map, next_r as i32, next_c as i32) {
			break;
		}
		if map[next_r][next_c] == Tile::DeepWater {
			break; 
		}
		map[next_r][next_c] = Tile::Lava;

		let (next_r, next_c) = pts_on_line(r, c, d, angle - 0.05); 
		if map::in_bounds(map, next_r as i32, next_c as i32) {
			map[next_r][next_c] = Tile::Lava;
		}

		let (next_r, next_c) = pts_on_line(r, c, d, angle + 0.05); 
		if map::in_bounds(map, next_r as i32, next_c as i32) {
			map[next_r][next_c] = Tile::Lava;
		}

		d += 1.0;

		let angle_delta = rand::thread_rng().gen_range(-0.05, 0.05);
		angle += angle_delta;
	}
}
	
fn generate_volcanic_island() -> Vec<Vec<Tile>> {
	let mut island = map::generate_mountainous_island();
	let mut snowpeaks;

	loop {
		snowpeaks = largest_contiguous_block(&island, &Tile::SnowPeak, 0, 0, 65);
		if snowpeaks.len() > 20 {
			break;
		}
		island = map::generate_mountainous_island();
	}

	let mut min_r = 999;
	let mut max_r = 0;
	let mut min_c = 999;
	let mut max_c = 0;
	for sq in snowpeaks {
		if sq.0 < min_r { min_r = sq.0 };
		if sq.0 > max_r { max_r = sq.0 };
		if sq.1 < min_c { min_c = sq.1 };
		if sq.1 > max_c { max_c = sq.1 };
	}
	let center_r = (min_r + max_r) / 2;
	let center_c = (min_c + max_c) / 2;

	for r in center_r - 1..=center_r + 1 {
		for c in center_c - 1..=center_c + 1 {
			island[r][c] = Tile::Lava;
		}
	}

	let num_of_flows = rand::thread_rng().gen_range(3, 6) + 2;
	for _ in 0..num_of_flows {
		draw_lava_flow(&mut island, center_r, center_c);
	}

	island
}

fn add_fruit(world_map: &Vec<Vec<Tile>>, 
				island_info: &IslandInfo,
				items: &mut ItemsTable) {
	let south_edge = island_info.offset_r + island_info.length;
	let east_edge = island_info.offset_c + island_info.length;

	// Let's make sure there's actually forests to place fruit on
	let mut found_tree = false;
	'outer: for r in island_info.offset_r..south_edge {
		for c in island_info.offset_c..east_edge {
			if world_map[r][c] == Tile::Tree {
				found_tree = true;
				break 'outer;
			}
		}
	}

	if !found_tree {
		return;
	}

	loop {
		let r = rand::thread_rng().gen_range(island_info.offset_r, south_edge);
		let c = rand::thread_rng().gen_range(island_info.offset_c, east_edge);

		let tile = &world_map[r][c];
		if *tile == Tile::Tree {
			let fruit = if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
				Item::get_item("coconut")	
			} else {
				Item::get_item("banana")	
			};
			
			items.add(r, c, fruit.unwrap());	
			break;
		}
	}
}

fn safe_to_place_item(tile: &Tile) -> bool {
	map::is_passable(tile) && *tile != Tile::Water && *tile != Tile::DeepWater
			&& *tile != Tile::Lava 
}

fn set_campsite(state: &mut GameState,
				island_info: &IslandInfo,	
				items: &mut ItemsTable) {

	let npcs = state.npcs.get_mut(&0).unwrap();
	let curr_map = state.map.get_mut(&0).unwrap();

	loop {
		let r = rand::thread_rng().gen_range(island_info.offset_r,
												island_info.offset_r + island_info.length);
		let c = rand::thread_rng().gen_range(island_info.offset_c, 
												island_info.offset_c + island_info.length);
		
		let tile = &curr_map[r][c];
		if safe_to_place_item(tile) {
			curr_map[r][c] = Tile::FirePit;

			let rum_count = rand::thread_rng().gen_range(0, 3);
			for _ in 0..rum_count {
				let delta = rnd_adj();
				let item_r = (r as i32 + delta.0) as usize;
				let item_c = (c as i32 + delta.1) as usize;
				let tile = &curr_map[item_r][item_c];
				if safe_to_place_item(tile) {
					let rum = Item::get_item("draught of rum").unwrap();
					items.add(item_r, item_c, rum);
				}	
			}	
			for _ in 0..rand::thread_rng().gen_range(1, 4) {
				loop {
					let delta = util::rnd_adj();
					let pirate_r = (r as i32 + delta.0) as usize;	
					let pirate_c = (c as i32 + delta.1) as usize;
	
					if !npcs.is_npc_at(pirate_r, pirate_c) {
						npcs.new_pirate(pirate_r, pirate_c, (r, c));
						break;	
					}

					break;
				}
			}
	
			break;
		}	
	}
}

fn get_castaway_line() -> String {
	let roll = rand::thread_rng().gen_range(0.0, 1.0);

	if roll < 0.2 {
		String::from("Wiiiiilsoooonnn!")
	} else if roll < 0.4 {
		String::from("Teetotalers are more apt to heed the mermaid's call.")
	} else if roll < 0.6 {
		String::from("Fresh springs in the mountains do a world of good.")
	} else if roll < 0.8 {
		String::from("The Yendorian Navy has a handsome bounty on pirates.")
	} else {
		String::from("A ghost ship haunts these waters at night.")
	}
}

// largely duplicated from the campsite code...
fn set_castaway(state: &mut GameState, island_info: &IslandInfo) {
	let npcs = state.npcs.get_mut(&0).unwrap();
	let curr_map = state.map.get_mut(&0).unwrap();
	loop {
		let r = rand::thread_rng().gen_range(island_info.offset_r,
												island_info.offset_r + island_info.length);
		let c = rand::thread_rng().gen_range(island_info.offset_c, 
												island_info.offset_c + island_info.length);
		
		let tile = &curr_map[r][c];
		if map::is_passable(&tile) && *tile != Tile::Water && *tile != Tile::DeepWater
				&& *tile != Tile::Lava {
			curr_map[r][c] = Tile::FirePit;
		
			let delta = util::rnd_adj();
			let castaway_r = (r as i32 + delta.0) as usize;	
			let castaway_c = (c as i32 + delta.1) as usize;

			if !npcs.is_npc_at(castaway_r, castaway_c) {
				npcs.new_castaway(castaway_r, castaway_c, (r, c), get_castaway_line());
				break;	
			}
            
			break;
		}	
	}
}

fn set_old_campsite(world_map: &mut Vec<Vec<Tile>>, island_info: &IslandInfo, items: &mut ItemsTable) {
	loop {
		let r = rand::thread_rng().gen_range(island_info.offset_r,
												island_info.offset_r + island_info.length);
		let c = rand::thread_rng().gen_range(island_info.offset_c, 
												island_info.offset_c + island_info.length);
		
		let tile = &world_map[r][c];
		if map::is_passable(tile) && *tile != Tile::Water && *tile != Tile::DeepWater
				&& *tile != Tile::Lava {
			world_map[r][c] = Tile::OldFirePit;

			let rum_count = rand::thread_rng().gen_range(0, 3) + 1;
			for _ in 0..rum_count {
				let delta = rnd_adj();
				let item_r = (r as i32 + delta.0) as usize;
				let item_c = (c as i32 + delta.1) as usize;
				let tile = &world_map[item_r][item_c];
				if safe_to_place_item(tile) {
					let rum = Item::get_item("draught of rum").unwrap();
					items.add(item_r, item_c, rum);
				}
			}	
			
			let pork_count = rand::thread_rng().gen_range(0, 2) + 1;
			for _ in 0..pork_count {
				let delta = rnd_adj();
				let item_r = (r as i32 + delta.0) as usize;
				let item_c = (c as i32 + delta.1) as usize;
				let tile = &world_map[item_r][item_c];
				if safe_to_place_item(tile) {
					let pork = Item::get_item("salted pork").unwrap();
					items.add(item_r, item_c, pork);
				}
			}	
			break;
		}	
	}
}

fn set_treasure_map(world_map: &Vec<Vec<Tile>>, island_info: &IslandInfo,
				items: &mut ItemsTable,
				cache: Vec<Item>, map_id: u8) -> Option<Item> {
	// Okay, I want to pick a random seacoast location and stick the treasure near
	// it. 
	//
	// A cooler way to do this might be to pathfind my way inland like a real
	// pirate might have but we'll save that for later

    let mut count = 0;
	loop {
		let j = rand::thread_rng().gen_range(0, island_info.coastline.len());
		let loc = island_info.coastline[j];	
		
		// I *could* probably figure out the centre of the island from
		// averaging the seacoast points and so focus my search on inland 
		// squares but I'd have to scratch my head over the geometry and this way
		// shouldn't take toooo long
		let r_delta = rand::thread_rng().gen_range(5, 10);
		let c_delta = rand::thread_rng().gen_range(5, 10);

		let tile = &world_map[loc.0 + r_delta][loc.1 + c_delta];
		if map::is_passable(tile) && *tile != Tile::Water && *tile != Tile::DeepWater {
			let nw_r = rand::thread_rng().gen_range(5, 15);
			let nw_c = rand::thread_rng().gen_range(10, 20);
			let actual_nw_r = ((loc.0 + r_delta) as i32 - nw_r) as usize;
			let actual_nw_c = ((loc.1 + c_delta) as i32 - nw_c) as usize;
			let actual_x_r = loc.0 + r_delta;
			let actual_x_c = loc.1 + c_delta;
			let map = Item::get_map((actual_nw_r, actual_nw_c), (actual_x_r, actual_x_c), map_id);
			for i in cache {
				items.add(actual_x_r, actual_x_c, i);
			}

			return Some(map);
		}

        // I can't imagine ever hitting this point...
        count += 1;
        if count > 100 {
            break;
        }
	}

	None
}

fn get_cache_items() -> Vec<Item> {
	let mut cache = Vec::new();

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		for _ in 0..rand::thread_rng().gen_range(0, 3) {
			let mut i = Item::get_item("draught of rum").unwrap();
			i.hidden = true;
			cache.push(i);
		}
	}

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		for _ in 0..rand::thread_rng().gen_range(0, 6) {
			let mut i = Item::get_item("lead ball").unwrap();
			i.hidden = true;
			cache.push(i);
		}
	} 

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.333 {
		for _ in 0..rand::thread_rng().gen_range(0, 12) {
			let mut i = Item::get_item("doubloon").unwrap();
			i.hidden = true;
			cache.push(i);
		}
	} 

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.10 {
		let mut i = Item::get_item("rusty cutlass").unwrap();
		i.hidden = true;
		cache.push(i);
	} 

	cache
}

fn good_for_fort(tile: &Tile) -> bool {
	if *tile == Tile::Tree || *tile == Tile::Grass ||
		*tile  == Tile::Sand || *tile == Tile::Dirt {
		true
	} else {
		false
	}
}

fn write_fort_sqs(loc: (usize, usize), world_map: &mut Vec<Vec<Tile>>,
			items: &mut ItemsTable) {
	let tile = if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		Tile::WoodWall
	} else {
		Tile::Wall
	};

	world_map[loc.0][loc.1] = tile.clone();
	world_map[loc.0][loc.1 + 1] = Tile::Window('-');
	world_map[loc.0][loc.1 + 2] = tile.clone();
	world_map[loc.0][loc.1 + 3] = tile.clone();
	world_map[loc.0 + 1][loc.1] = Tile::Window('|');
	world_map[loc.0 + 1][loc.1 + 3] = tile.clone();
	world_map[loc.0 + 2][loc.1] = tile.clone();
	world_map[loc.0 + 2][loc.1 + 3] = tile.clone();
	world_map[loc.0 + 2][loc.1 + 4] = tile.clone();
	world_map[loc.0 + 2][loc.1 + 5] = tile.clone();
	world_map[loc.0 + 2][loc.1 + 6] = tile.clone();
	world_map[loc.0 + 2][loc.1 + 7] = tile.clone();
	world_map[loc.0 + 3][loc.1] = tile.clone();
	world_map[loc.0 + 3][loc.1 + 1] = tile.clone();
	world_map[loc.0 + 3][loc.1 + 3] = tile.clone();
	world_map[loc.0 + 3][loc.1 + 7] = tile.clone();
	world_map[loc.0 + 4][loc.1 + 1] = tile.clone();
	world_map[loc.0 + 4][loc.1 + 7] = tile.clone();
	world_map[loc.0 + 5][loc.1 + 1] = tile.clone();
	world_map[loc.0 + 5][loc.1 + 7] = tile.clone();
	world_map[loc.0 + 6][loc.1 + 1] = tile.clone();
	world_map[loc.0 + 6][loc.1 + 2] = tile.clone();
	world_map[loc.0 + 6][loc.1 + 5] = tile.clone();
	world_map[loc.0 + 6][loc.1 + 6] = tile.clone();
	world_map[loc.0 + 6][loc.1 + 7] = tile.clone();

	world_map[loc.0 + 1][loc.1 + 1] = Tile::Floor;
	world_map[loc.0 + 1][loc.1 + 2] = Tile::Floor;
	world_map[loc.0 + 2][loc.1 + 1] = Tile::Floor;
	world_map[loc.0 + 2][loc.1 + 2] = Tile::Floor;
	world_map[loc.0 + 3][loc.1 + 2] = Tile::Floor;
	world_map[loc.0 + 3][loc.1 + 4] = Tile::Floor;
	world_map[loc.0 + 3][loc.1 + 5] = Tile::Floor;
	world_map[loc.0 + 3][loc.1 + 6] = Tile::Floor;
	world_map[loc.0 + 4][loc.1 + 2] = Tile::Floor;
	world_map[loc.0 + 4][loc.1 + 3] = Tile::Floor;
	world_map[loc.0 + 4][loc.1 + 4] = Tile::Floor;
	world_map[loc.0 + 4][loc.1 + 5] = Tile::Floor;
	world_map[loc.0 + 4][loc.1 + 6] = Tile::Floor;
	world_map[loc.0 + 5][loc.1 + 2] = Tile::Floor;
	world_map[loc.0 + 5][loc.1 + 3] = Tile::Floor;
	world_map[loc.0 + 5][loc.1 + 4] = Tile::Mast('|');
	world_map[loc.0 + 5][loc.1 + 5] = Tile::Floor;
	world_map[loc.0 + 5][loc.1 + 6] = Tile::Floor;

	world_map[loc.0 + 7][loc.1 + 2] = Tile::Mast('/');

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.99 {
		let c = rand::thread_rng().gen_range(0, 5);
		let r = rand::thread_rng().gen_range(0, 2);

		if rand::thread_rng().gen_range(0.0, 1.0) < 0.2 {
			let mut i = Item::get_item("corroded flintlock").unwrap();
			i.hidden = true;
			items.add(loc.0 + 4 + r, loc.1 + 2 + c, i);
		}
		for _ in 0..rand::thread_rng().gen_range(0, 5) {
			let mut i = Item::get_item("lead ball").unwrap();
			i.hidden = true;
			items.add(loc.0 + 4 + r, loc.1 + 2 + c, i);
		}
		for _ in 0..rand::thread_rng().gen_range(0, 3) {
			let mut i = Item::get_item("draught of rum").unwrap();
			i.hidden = true;
			items.add(loc.0 + 4 + r, loc.1 + 2 + c, i);
		}
	}
}

fn place_fort(world_map: &mut Vec<Vec<Tile>>,
			island_info: &IslandInfo,
			items: &mut ItemsTable) {

	// Find all grass, dirt, sand or trees
	let mut potentials = VecDeque::new();
	for r in island_info.offset_r..island_info.offset_r+island_info.length-8 {
		for c in island_info.offset_c..island_info.offset_c+island_info.length-8 {
			if good_for_fort(&world_map[r][c]) {
				potentials.push_back((r, c));
			}
		}
	}

	let mut count = 0;
	while count < 20 {
		let loc = rand::thread_rng().gen_range(0, potentials.len());
		let sq = potentials[loc];
		
		let mut good_sqs = 0;
		for r in sq.0..sq.0+8 {
			for c in sq.1..sq.1+8 {
				if world_map[r][c] == Tile::Mountain || world_map[r][c] == Tile::SnowPeak {
					good_sqs -= 10;
				}
				if good_for_fort(&world_map[r][c]) {
					good_sqs += 1;
				}
			}
		}

		if good_sqs > 10 {
			write_fort_sqs(sq, world_map, items);
			break;
		}	

		count += 1;
	}	
}

fn add_shipwreck(state: &mut GameState,
			island_info: &IslandInfo,
			items: &mut ItemsTable,
			cache: Vec<Item>,
			guarantee_cache: bool) -> String {
	let curr_map = state.map.get_mut(&0).unwrap();
	let loc = rand::thread_rng().gen_range(0, island_info.coastline.len());
	let centre = island_info.coastline[loc];	

	let wreck_name = ship::random_name(true);
	let deck = Tile::Shipwreck(ship::DECK_ANGLE, wreck_name.clone()); 
	curr_map[centre.0][centre.1] = deck;

	let r = dice::roll(3, 1, 0);
	let mast_ch = if r == 1 { '|' }
					else if r == 2 { '\\' }
					else { '/' };
	let mast_loc = rnd_adj();
	let mast_r = (centre.0 as i32 + mast_loc.0) as usize;
	let mast_c = (centre.1 as i32 + mast_loc.1) as usize;
	curr_map[mast_r][mast_c] = Tile::Mast(mast_ch);

	loop {
		let part_loc = rnd_adj();
		if part_loc != mast_loc {
			let r = dice::roll(2, 1, 0);
			if r == 1 {
				let part_r = (centre.0 as i32 + part_loc.0) as usize;
				let part_c = (centre.1 as i32 + part_loc.1) as usize;
				curr_map[part_r][part_c] = Tile::Mast(ship::DECK_ANGLE);
			} else {
				let part_r = (centre.0 as i32 + part_loc.0) as usize;
				let part_c = (centre.1 as i32 + part_loc.1) as usize;
				curr_map[part_r][part_c] = Tile::Mast(ship::DECK_STRAIGHT);
			}

			// chance of there being a hidden cache
			if guarantee_cache || rand::thread_rng().gen_range(0.0, 1.0) < 0.50 {
				let loc_r = (centre.0 as i32 + part_loc.0) as usize;
				let loc_c = (centre.1 as i32 + part_loc.1) as usize;
				for i in cache {
					items.add(loc_r, loc_c, i);
				}
			}

			break;
		}
	}
		
	let part_loc = rnd_adj();
	let r = dice::roll(4, 1, 0);
	if r == 1 {
		let part_r = (centre.0 as i32 + part_loc.0 * 2) as usize;
		let part_c = (centre.1 as i32 + part_loc.1 * 2) as usize;
		curr_map[part_r][part_c] = Tile::ShipPart(ship::BOW_NE);
	} else if r == 2 {
		let part_r = (centre.0 as i32 + part_loc.0 * 2) as usize;
		let part_c = (centre.1 as i32 + part_loc.1 * 2) as usize;
		curr_map[part_r][part_c] = Tile::Mast(ship::BOW_NW);
	} else if r == 3 {
		let part_r = (centre.0 as i32 + part_loc.0 * 2) as usize;
		let part_c = (centre.1 as i32 + part_loc.1 * 2) as usize;
		curr_map[part_r][part_c] = Tile::Mast(ship::BOW_SE);
	} else if r == 3 {
		let part_r = (centre.0 as i32 + part_loc.0 * 2) as usize;
		let part_c = (centre.1 as i32 + part_loc.1 * 2) as usize;
		curr_map[part_r][part_c] = Tile::Mast(ship::BOW_SW);
	}

	// merfolk like to hang out near shipwrecks
 	if rand::thread_rng().gen_range(0.0, 1.0) < 0.20 {
		let count = rand::thread_rng().gen_range(1, 3);
		for _ in 0..count {
			place_mermaid(state, centre);
		}
	}

	wreck_name
}

fn place_mermaid(state: &mut GameState, loc: (usize, usize)) {
	let npcs = state.npcs.get_mut(&0).unwrap();
	loop {
		let delta_r = rand::thread_rng().gen_range(-5, 6);
		let delta_c = rand::thread_rng().gen_range(-5, 6);
		let mer_r = (loc.0 as i32 + delta_r) as usize;
		let mer_c = (loc.1 as i32 + delta_c) as usize;
        
		if map::in_bounds(&state.map[&0], mer_r as i32, mer_c as i32) &&
			(state.map[&0][mer_r][mer_c] == Tile::Water ||
				state.map[&0][mer_r][mer_c] == Tile::DeepWater)	&&
			    !npcs.is_npc_at(mer_r, mer_c) {
                npcs.new_merperson(mer_r, mer_c);
                return;
		}
	}
}


fn place_spring(state: &mut GameState, island_info: &IslandInfo) {
	let trees = largest_contiguous_block(&state.map[&0], &Tile::Tree, island_info.offset_r,
							island_info.offset_c, island_info.length); 

	let curr_map = state.map.get_mut(&0).unwrap();	
	if trees.len() > 0 {
		let mut candidates = Vec::new();
		for tree in trees {
			let mut count = 0;
			if curr_map[tree.0 - 1][tree.1] == Tile::Mountain { count += 1; }
			if curr_map[tree.0 + 1][tree.1] == Tile::Mountain { count += 1; }
			if curr_map[tree.0][tree.1 - 1] == Tile::Mountain { count += 1; }
			if curr_map[tree.0][tree.1 + 1] == Tile::Mountain { count += 1; }
			if count > 1 {
				candidates.push(tree);
			}
		}

		if candidates.len() > 0 {
			let roll = rand::thread_rng().gen_range(0, candidates.len());
			curr_map[candidates[roll].0][candidates[roll].1] = Tile::Spring;
		}
	}	
}

// Some map analytics functions
fn is_hidden_valley(map: &Vec<Vec<Tile>>, r: usize, c: usize) -> HashSet<(usize, usize)> {
	let mut valley = HashSet::new();
	let mut queue = VecDeque::new();
	queue.push_back((r, c));

	while queue.len() > 0 {
		let loc = queue.pop_front().unwrap();
		valley.insert(loc);

		for r in -1..=1 {
			for c in -1..=1 {
				if r == 0 && c == 0 { continue; }
				let nr = (loc.0 as i32 + r) as usize;
				let nc = (loc.1 as i32 + c) as usize;

				if !map::in_bounds(map, nr as i32, nc as i32) {
					return HashSet::new();
				}

				if map[nr][nc] != Tile::Tree && map[nr][nc] != Tile::Mountain 
						&& map[nr][nc] != Tile::SnowPeak {
					return HashSet::new();
				}

				if map[nr][nc] == Tile::Tree && !valley.contains(&(nr, nc)) {
					queue.push_back((nr, nc));
				}
			}
		}
	}

	valley
}

// Sometimes the map generator will create pockets of (almost
// always forest) inside mountain ranges, completely cut off.
// I thought it would be fun to find them and use them if they 
// exist.
//
// Look for any blocks of trees where all their neighbours are 
// either trees, mountains or snow peeaks. (And maybe I should 
// include lava?) Another floodfill type search...
fn find_hidden_valleys(map: &Vec<Vec<Tile>>, island_info: &IslandInfo) -> HashSet<(usize, usize)> {
	let mut valleys = HashSet::new();

	for r in island_info.offset_r..island_info.length + island_info.offset_r {
        for c in island_info.offset_c..island_info.length + island_info.offset_c {
            if valleys.contains(&(r, c)) {
                continue;
            }

			if map[r][c] == Tile::Tree {
				let valley = is_hidden_valley(map, r, c);
                valleys.extend(valley);
			}
		}
	}	

    valleys
}

// Since the maps can be generated sometimes small (especially
// the atoll type) and ceneterd, find the NW square closest to
// the island where the row and column is still all open water
fn find_nearest_clear_nw(map: &Vec<Vec<Tile>>) -> (usize, usize) {
	let mut nw = (0, 0);

	loop {
		nw.0 += 1;
		nw.1 += 1;
		
		for c in nw.1..map.len() {
			if map[nw.0][c] != Tile::Water && map[nw.0][c] != Tile::DeepWater {
				return (nw.0 - 1, nw.1 - 1);
			}
		}
		for r in nw.0..map.len() {
			if map[r][nw.1] != Tile::Water && map[r][nw.1] != Tile::DeepWater {
				return (nw.0 - 1, nw.1 - 1);
			}
		}

        break;
	}

	(0, 0)
}

fn flood_fill_search(map: &Vec<Vec<Tile>>, target: &Tile, r: usize, c: usize) 
		-> HashSet<(usize, usize)> {
	let mut block = HashSet::new();
	let mut queue = VecDeque::new();
	queue.push_back((r, c));
	
	while queue.len() > 0 {
		let curr = queue.pop_front().unwrap();
		block.insert((curr.0, curr.1));
		
		for r in -1..=1 {
			for c in -1..=1 {
				if r == 0 && c == 0 { continue; }
				if !map::in_bounds(map, curr.0 as i32 + r, curr.1 as i32 + c) {
					continue;
				}
				let nr = (curr.0 as i32 + r) as usize;
				let nc = (curr.1 as i32 + c) as usize;

				if map[nr][nc] != *target || block.contains(&(nr, nc)) {
					continue;
				}

				block.insert((nr, nc));
				queue.push_back((nr, nc));
			}
		}
	}

	block	
}

// Floodfill to find the largest block of a given tile type
fn largest_contiguous_block(map: &Vec<Vec<Tile>>, target: &Tile,
		offset_r: usize, offset_c: usize, length: usize) -> HashSet<(usize, usize)> {
	let mut targets_found: HashSet<(usize, usize)> = HashSet::new();
	let mut best = HashSet::new();

	for r in offset_r..offset_r+length {
		for c in offset_c..offset_c+length {
			if map[r][c] == *target {
				if !targets_found.contains(&(r, c)) {
					let block = flood_fill_search(map, target, r, c);
					for sq in block.clone() {
						targets_found.insert((sq.0, sq.1));
					}

					if block.len() > best.len() {
						best = block;
					}
				}
			}
		}
	}

	best
}

// Yep, our old pal floodfill again
fn find_coastline(world_map: &Vec<Vec<Tile>>, island_info: &mut IslandInfo) {
	let mut queue = VecDeque::new();
	let mut visited = HashSet::new();

	// Sometimes the island generator does write land on the very edge
	// of the map so make sure we're actually starting on an ocean square
	for c in island_info.offset_c..island_info.offset_c + island_info.length {
		if world_map[island_info.offset_r][c] == Tile::DeepWater {
			queue.push_back((island_info.offset_r, c));
			visited.insert((island_info.offset_r, c));
			break;
		}
	}

	let south_edge = (island_info.offset_r + island_info.length) as i32;
	let east_edge = (island_info.offset_c + island_info.length) as i32;
	while queue.len() > 0 {
		let curr = queue.pop_front().unwrap();
	
		for r in -1..=1 {
			for c in -1..=1 {
				let nr = curr.0 as i32 + r;
				let nc = curr.1 as i32 + c;

				if nr < island_info.offset_r as i32 || nr >= south_edge { continue }	
				if nc < island_info.offset_c as i32 || nc >= east_edge { continue }	
				
				let loc = (nr as usize, nc as usize);
				if world_map[nr as usize][nc as usize] != Tile::DeepWater 
						&& world_map[nr as usize][nc as usize] != Tile::Water {
					island_info.coastline.push_back(loc);
				} else if !visited.contains(&loc) {
					visited.insert(loc);
					queue.push_back(loc);
				}
			}
		}	
	}
}

// Should probably switch to a more generic "Find any of these tile types that are accessible from
// the shore"
fn mountains_reachable_by_shore(map: &Vec<Vec<Tile>>, island_info: &IslandInfo) -> VecDeque<(usize, usize)> {
    let mut reachable = VecDeque::new();

    let hidden_valleys = find_hidden_valleys(map, island_info);

    // Criteria for a mountain that's reachable from the shore is:
    // any mountain that isn't completely surrounded by mountains or 
    // locations in hidden valleys
    for r in island_info.offset_r..island_info.length + island_info.offset_r {
        for c in island_info.offset_c..island_info.length + island_info.offset_c {
            if map[r][c] != Tile::Mountain { continue; }
            if hidden_valleys.contains(&(r, c)) { continue; }
            
            if (map[r - 1][c] == Tile::Mountain || map[r - 1][c] == Tile::SnowPeak) &&
                (map[r + 1][c] == Tile::Mountain || map[r + 1][c] == Tile::SnowPeak) &&
                (map[r][c - 1] == Tile::Mountain || map[r][c - 1] == Tile::SnowPeak) &&
                (map[r][c + 1] == Tile::Mountain || map[r][c + 1] == Tile::SnowPeak) {
                continue;
            }

            reachable.push_back((r, c));
        }
    }
    
    reachable
}

// This assumes the caves generated are always fully connected...
fn find_cave_exit(cave_map: &Vec<Vec<Tile>>, length: usize, width: usize) -> (usize, usize) {
    let roll = rand::thread_rng().gen_range(0.0, 1.0);
    if roll < 0.5 {
        let col = rand::thread_rng().gen_range(2, width - 2);
        if roll < 0.25 {
            for row in 2..length - 2 {
                if cave_map[row][col] != Tile::Wall {
                    return (row - 1, col);
                }
            }
        } else {
            for row in (2..length - 2).rev() {
                if cave_map[row][col] != Tile::Wall {
                    return (row + 1, col);
                }
            }
        }
    } else {
        let row = rand::thread_rng().gen_range(2, length - 2);
        if roll < 0.5 {
            for col in 2..width - 2 {
                if cave_map[row][col] != Tile::Wall {
                    return (row, col - 1);
                }
            }
        } else {
            for col in (2..width - 2).rev() {
                if cave_map[row][col] != Tile::Wall {
                    return (row, col + 1);
                }
            }
        }
    }

    (0, 0)
}

fn place_cave(state: &mut GameState, 
			items: &mut HashMap<u8, ItemsTable>, 
			island_info: &IslandInfo,
			ships: &mut HashMap<u8, ShipsTable>) {
    let reachable = mountains_reachable_by_shore(&state.map[&state.map_id], island_info);
    let next_map_id = state.map.len() as u8;
    let curr_map = state.map.get_mut(&state.map_id).unwrap();
	let cave_length = 20;
	let cave_width = 30;

    if reachable.len() > 0 {
        let cave_loc_id = rand::thread_rng().gen_range(0, reachable.len());
        let cave_loc = reachable[cave_loc_id];
        curr_map[cave_loc.0][cave_loc.1] = Tile::Portal((cave_loc.0, cave_loc.1, 1));
        println!("{:?}", cave_loc);

        let mut cave_map = map::generate_cave(cave_width, cave_length);

        let exit = find_cave_exit(&cave_map, cave_length, cave_width);
        if exit.0 != 0 && exit.1 != 0 {
            cave_map[exit.0][exit.1] = Tile::Portal((cave_loc.0, cave_loc.1, state.map_id));
            curr_map[cave_loc.0][cave_loc.1] = Tile::Portal((exit.0, exit.1, next_map_id));
            state.map.insert(next_map_id, cave_map);

            state.npcs.insert(next_map_id, NPCTracker::new());
            items.insert(next_map_id, ItemsTable::new());
			ships.insert(next_map_id, ShipsTable::new());
            state.weather.insert(next_map_id, Weather::new());
        }

		for _ in 0..3 {
			loop {
				let r = rand::thread_rng().gen_range(0, cave_length); 
				let c = rand::thread_rng().gen_range(0, cave_width); 
				if state.map.get_mut(&next_map_id).unwrap()[r][c] == Tile::StoneFloor {
					state.npcs.get_mut(&next_map_id).unwrap().new_rat(r, c);
					break;
				}
			}
		}
    }
}

