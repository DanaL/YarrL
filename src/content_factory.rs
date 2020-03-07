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

use super::{GameState, ItemsTable, Map};
use crate::dice;
use crate::items::Item;
use crate::map;
use crate::map::Tile;
use crate::ship;
use crate::ship::Ship;
use crate::util;
use crate::util::NameSeeds;
use crate::util::rnd_adj;

pub const WORLD_WIDTH: usize = 250;
pub const WORLD_HEIGHT: usize = 250;

fn initialize_map(map: &mut Map) {
	let mut top = Vec::new();
	for _ in 0..WORLD_WIDTH {
		top.push(Tile::WorldEdge);
	}
	map.push(top);

	for _ in 0..WORLD_HEIGHT - 2 {
		let mut row = Vec::new();
		row.push(Tile::WorldEdge);
		for _ in 0..WORLD_WIDTH - 2 {
			row.push(Tile::DeepWater);
		}
		row.push(Tile::WorldEdge);
		map.push(row);
	}

	let mut bottom = Vec::new();
	for _ in 0..WORLD_WIDTH {
		bottom.push(Tile::WorldEdge);
	}
	map.push(bottom);
}

pub fn generate_world(state: &mut GameState,
		items: &mut ItemsTable,
		ships: &mut HashMap<(usize, usize), Ship>) {

	initialize_map(&mut state.map);

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

	let q1_seacoast = create_island(&mut state.map, items, 5, 5);
	let q2_seacoast = create_island(&mut state.map, items, 10, 100);
	let q3_seacoast = create_island(&mut state.map, items, 100, 10);
	let q4_seacoast = create_island(&mut state.map, items, 100, 100);
	let coasts = vec![q1_seacoast, q2_seacoast, q3_seacoast, q4_seacoast];
	let offsets = vec![(5, 5), (10, 100), (100, 10), (100,100)];

	state.pirate_lord = get_pirate_lord();
	state.player_ship = ship::random_name();
	state.starter_clue = clue_1;

	let eye_patch = Item::get_item("magic eyepatch").unwrap();
	let mut c = Vec::new();
	c.push(eye_patch);
	let hint_to_final_clue;

	// We also need to include the location of the treasure along with the 
	// eye patch
	if final_clue == 0 {
		let roll = rand::thread_rng().gen_range(0, 4);
		hint_to_final_clue = set_treasure_map(&state.map, &coasts[roll],
			items, offsets[roll].0, offsets[roll].1, c).unwrap();
	} else {
		let roll = rand::thread_rng().gen_range(0, 4);
		let ship_name = add_shipwreck(&mut state.map, &coasts[roll],
			items, offsets[roll].0, offsets[roll].1, c);
		hint_to_final_clue = Item::get_note(state.note_count);
		state.notes.insert(state.note_count, Item::get_note_text(&ship_name));
		state.note_count += 1;
	}

	// Now the second clue
	let mut c = Vec::new();
	c.push(hint_to_final_clue);
	let hint_to_2nd_clue;

	if clue_2 == 0 {
		let roll = rand::thread_rng().gen_range(0, 4);
		hint_to_2nd_clue = set_treasure_map(&state.map, &coasts[roll],
			items, offsets[roll].0, offsets[roll].1, c).unwrap();
	} else {
		let roll = rand::thread_rng().gen_range(0, 4);
		let ship_name = add_shipwreck(&mut state.map, &coasts[roll],
			items, offsets[roll].0, offsets[roll].1, c);
		hint_to_2nd_clue = Item::get_note(state.note_count);
		state.notes.insert(state.note_count, Item::get_note_text(&ship_name));
		state.note_count += 1;
	}

	// Now the first clue
	let mut c = Vec::new();
	c.push(hint_to_2nd_clue);
	if clue_1 == 0 {
		let roll = rand::thread_rng().gen_range(0, 4);
		let map = set_treasure_map(&state.map, &coasts[roll],
			items, offsets[roll].0, offsets[roll].1, c).unwrap();
		state.player.inventory.add(map);
	} else {
		let roll = rand::thread_rng().gen_range(0, 4);
		let ship_name = add_shipwreck(&mut state.map, &coasts[roll],
			items, offsets[roll].0, offsets[roll].1, c);
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
	ships.insert((state.player.row, state.player.col), ship);
}

fn create_island(map: &mut Vec<Vec<Tile>>, 
					items: &mut ItemsTable,
					offset_r: usize,
					offset_c: usize) -> VecDeque<(usize, usize)> {
	let mut island;
	let island_type = rand::thread_rng().gen_range(0.0, 1.0);
	let mut max_shipwrecks = 0;
	let mut max_campsites = 0;
	let mut max_fruit = 0;

	if island_type < 0.0 {
		// regular island
		island = map::generate_std_island();
		max_shipwrecks = 2;
		max_campsites = 4;
		max_fruit = 8;		
	} else if island_type < 0.85 {
		// atoll
		island = map::generate_atoll();
		max_shipwrecks = 6;
		max_campsites = 2;
		max_fruit = 3;
	} else {
		// volcano
		island = generate_volcanic_island();
		max_shipwrecks = 3;
		max_campsites = 2;
		max_fruit = 6;
	}

	// find_hidden_valleys(&island);
	let seacoast = find_all_seacoast(&island);
	for _ in 0..rand::thread_rng().gen_range(0, max_shipwrecks) {
		let cache = get_cache_items();
		add_shipwreck(&mut island, &seacoast, items, offset_r, offset_c, cache);
	}
	for _ in 0..rand::thread_rng().gen_range(0, max_campsites) {
		set_campsite(&mut island, items, offset_r, offset_c);
	}
	for _ in 0..rand::thread_rng().gen_range(0, max_fruit) {
		add_fruit(&island, items, offset_r, offset_c);
	}

	let nw = find_nearest_clear_nw(&island);
	for r in nw.0..island.len() {
		for c in nw.1..island.len() {
			map[r + offset_r][c + offset_c] = island[r][c].clone();
		}
	}

	seacoast
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
		snowpeaks = largest_contiguous_block(&island, &Tile::SnowPeak);
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

fn add_fruit(map: &Vec<Vec<Tile>>, items: &mut ItemsTable,
				world_offset_r: usize,
				world_offset_c: usize) {

	// Let's make sure there's actually forests to place fruit on
	let mut found_tree = false;
	'outer: for r in 0..map.len() {
		for c in 0..map.len() {
			if map[r][c] == Tile::Tree {
				found_tree = true;
				break 'outer;
			}
		}
	}
	if !found_tree {
		return;
	}

	loop {
		let r = rand::thread_rng().gen_range(0, map.len());
		let c = rand::thread_rng().gen_range(0, map.len());

		let tile = &map[r][c];
		if *tile == Tile::Tree {
			let fruit = if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
				Item::get_item("coconut")	
			} else {
				Item::get_item("banana")	
			};
			
			items.add(r + world_offset_r, c + world_offset_c, fruit.unwrap());	
			break;
		}
	}
}

fn set_campsite(map: &mut Vec<Vec<Tile>>, items: &mut ItemsTable,
				world_offset_r: usize,
				world_offset_c: usize) {
	
	loop {
		let r = rand::thread_rng().gen_range(0, map.len());
		let c = rand::thread_rng().gen_range(0, map.len());
		
		let tile = &map[r][c];
		if map::is_passable(tile) && *tile != Tile::Water && *tile != Tile::DeepWater
				&& *tile != Tile::Lava {
			map[r][c] = Tile::OldFirePit;
		
			let actual_r = r + world_offset_r;
			let actual_c = c + world_offset_c;

			let rum_count = rand::thread_rng().gen_range(0, 3) + 1;
			for _ in 0..rum_count {
				let delta = rnd_adj();
				let rum = Item::get_item("draught of rum").unwrap();
				items.add((actual_r as i32 + delta.0) as usize, 
						(actual_c as i32 + delta.1) as usize, rum);
			}	
			
			let pork_count = rand::thread_rng().gen_range(0, 2) + 1;
			for _ in 0..pork_count {
				let delta = rnd_adj();
				let pork = Item::get_item("salted pork").unwrap();
				items.add((actual_r as i32 + delta.0) as usize, 
						(actual_c as i32 + delta.1) as usize, pork);
			}	
			break;
		}	
	}
}

fn set_treasure_map(map: &Vec<Vec<Tile>>, seacoast: &VecDeque<(usize, usize)>, 
				items: &mut ItemsTable,
				world_offset_r: usize,
				world_offset_c: usize,
				cache: Vec<Item>) -> Option<Item> {
	// Okay, I want to pick a random seacoast location and stick the treasure near
	// it. 
	//
	// A cooler way to do this might be to pathfind my way inland like a real
	// pirate might have but we'll save that for later

	loop {
		let j = rand::thread_rng().gen_range(0, seacoast.len());
		let loc = seacoast[j];	
		
		// I *could* probably figure out the centre of the island from
		// averaging the seacoast points and so focus my search on inland 
		// squares but I'd have to scratch my head over the geometry and this way
		// shouldn't take toooo long
		let r_delta = rand::thread_rng().gen_range(5, 10);
		let c_delta = rand::thread_rng().gen_range(5, 10);

		let tile = &map[loc.0 + r_delta][loc.1 + c_delta];
		if map::is_passable(tile) && *tile != Tile::Water && *tile != Tile::DeepWater {
			let nw_r = rand::thread_rng().gen_range(5, 15);
			let nw_c = rand::thread_rng().gen_range(10, 20);
			let actual_nw_r = ((loc.0 + r_delta + world_offset_r) as i32 - nw_r) as usize;
			let actual_nw_c = ((loc.1 + c_delta + world_offset_c) as i32 - nw_c) as usize;
			let actual_x_r = loc.0 + r_delta + world_offset_r;
			let actual_x_c = loc.1 + c_delta + world_offset_c;
			let map = Item::get_map((actual_nw_r, actual_nw_c), (actual_x_r, actual_x_c));
			for i in cache {
				items.add(actual_x_r, actual_x_c, i);
			}

			return Some(map);
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

fn add_shipwreck(map: &mut Vec<Vec<Tile>>, seacoast: &VecDeque<(usize, usize)>,
			items: &mut ItemsTable,
			world_offset_r: usize,
			world_offset_c: usize,
			cache: Vec<Item>) -> String {
	let loc = rand::thread_rng().gen_range(0, seacoast.len());
	let centre = seacoast[loc];	

	let wreck_name = ship::random_name();
	let deck = Tile::Shipwreck(ship::DECK_ANGLE, wreck_name.clone()); 
	map[centre.0][centre.1] = deck;

	let r = dice::roll(3, 1, 0);
	let mast_ch = if r == 1 { '|' }
					else if r == 2 { '\\' }
					else { '/' };
	let mast_loc = rnd_adj();
	let mast_r = (centre.0 as i32 + mast_loc.0) as usize;
	let mast_c = (centre.1 as i32 + mast_loc.1) as usize;
	map[mast_r][mast_c] = Tile::Mast(mast_ch);

	loop {
		let part_loc = rnd_adj();
		if part_loc != mast_loc {
			let r = dice::roll(2, 1, 0);
			if r == 1 {
				let part_r = (centre.0 as i32 + part_loc.0) as usize;
				let part_c = (centre.1 as i32 + part_loc.1) as usize;
				map[part_r][part_c] = Tile::Mast(ship::DECK_ANGLE);
			} else {
				let part_r = (centre.0 as i32 + part_loc.0) as usize;
				let part_c = (centre.1 as i32 + part_loc.1) as usize;
				map[part_r][part_c] = Tile::Mast(ship::DECK_STRAIGHT);
			}

			// chance of there being a hidden cache
			if rand::thread_rng().gen_range(0.0, 1.0) < 0.50 {
				let loc_r = (centre.0 as i32 + part_loc.0) as usize + world_offset_r;
				let loc_c = (centre.1 as i32 + part_loc.1) as usize + world_offset_c;
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
		map[part_r][part_c] = Tile::ShipPart(ship::BOW_NE);
	} else if r == 2 {
		let part_r = (centre.0 as i32 + part_loc.0 * 2) as usize;
		let part_c = (centre.1 as i32 + part_loc.1 * 2) as usize;
		map[part_r][part_c] = Tile::Mast(ship::BOW_NW);
	} else if r == 3 {
		let part_r = (centre.0 as i32 + part_loc.0 * 2) as usize;
		let part_c = (centre.1 as i32 + part_loc.1 * 2) as usize;
		map[part_r][part_c] = Tile::Mast(ship::BOW_SE);
	} else if r == 3 {
		let part_r = (centre.0 as i32 + part_loc.0 * 2) as usize;
		let part_c = (centre.1 as i32 + part_loc.1 * 2) as usize;
		map[part_r][part_c] = Tile::Mast(ship::BOW_SW);
	}

	wreck_name
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
fn find_hidden_valleys(map: &Vec<Vec<Tile>>) {
	//let valleys = Vec::new();

	for r in 0..map.len() {
		for c in 0..map.len() {
			if map[r][c] == Tile::Tree {
				let c = is_hidden_valley(map, r, c);
				if c.len() > 0 {
					println!("found a hidden valley!");
					println!("{:?}", c);
				}
			}
		}
	}	
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
fn largest_contiguous_block(map: &Vec<Vec<Tile>>, target: &Tile) -> HashSet<(usize, usize)> {
	let mut targets_found: HashSet<(usize, usize)> = HashSet::new();
	let mut best = HashSet::new();

	'fuck: for r in 0..map.len() {
		for c in 0..map.len() {
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
fn find_all_seacoast(map: &Vec<Vec<Tile>>) -> VecDeque<(usize, usize)> {
	let mut queue = VecDeque::new();
	let mut visited = HashSet::new();
	let mut seacoast = VecDeque::new();

	// Sometimes the island generator does write land on the very edge
	// of the map so make sure we're actually starting on an ocean square
	for c in 0..map.len() {
		if map[0][c] == Tile::DeepWater {
			queue.push_back((0, c));
			visited.insert((0, c));
			break;
		}
	}

	while queue.len() > 0 {
		let curr = queue.pop_front().unwrap();
	
		for r in -1..=1 {
			for c in -1..=1 {
				let nr = curr.0 as i32 + r;
				let nc = curr.1 as i32 + c;
	
				if !map::in_bounds(&map, nr, nc) { continue; }
				
				let loc = (nr as usize, nc as usize);
				if map[nr as usize][nc as usize] != Tile::DeepWater 
						&& map[nr as usize][nc as usize] != Tile::Water {
					seacoast.push_back(loc);
				} else if !visited.contains(&loc) {
					visited.insert(loc);
					queue.push_back(loc);
				}
			}
		}	
	}

	seacoast
}
