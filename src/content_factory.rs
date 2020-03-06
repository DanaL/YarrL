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
use crate::util::rnd_adj;

pub const WORLD_WIDTH: usize = 250;
pub const WORLD_HEIGHT: usize = 150;

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
	
	// make an island, copy it to the top left quadrant
	//let island = map::generate_atoll();
	//for r in nw.0..island.len() {
	//	for c in nw.1..island.len() {
	//		state.map[r + 5][c + 5] = island[r][c];
	//	}
	//}

	let mut island = generate_volcanic_island();
	let nw = find_nearest_clear_nw(&island);
	find_hidden_valleys(&island);
	let seacoast = find_all_seacoast(&island);
	add_shipwreck(&mut island, &seacoast, items, 5, 5);

	for r in nw.0..island.len() {
		for c in nw.1..island.len() {
			state.map[r + 5][c + 5] = island[r][c].clone();
		}
	}

	let mut island = map::generate_atoll();
	let seacoast = find_all_seacoast(&island);
	for _ in 0..3 {
		add_shipwreck(&mut island, &seacoast, items, 2, 100);
	}

	for r in 0..island.len() {
		for c in 0..island.len() {
			state.map[r+2][c + 100] = island[r][c].clone();
		}
	}

	// place the player
	state.player.on_ship = true;
	state.player.bearing = 6;
	state.player.wheel = 0;
	state.player.row = 5;
	state.player.col = 5;

	let map = Item::get_map((15, 15), (22, 20));
	state.player.inventory.add(map);

	let mut ship = Ship::new(ship::random_name());
	ship.row = state.player.row;
	ship.col = state.player.col;
	ship.bearing = 6;
	ship.wheel = 0;
	ship.update_loc_info();
	ships.insert((state.player.row, state.player.col), ship);
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

fn add_cache(items: &mut ItemsTable, row: usize, col: usize) {
	if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		for _ in 0..rand::thread_rng().gen_range(0, 3) {
			let mut i = Item::get_item("draught of rum").unwrap();
			i.hidden = true;
			items.add(row, col, i);
		}
	}

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.5 {
		for _ in 0..rand::thread_rng().gen_range(0, 6) {
			let mut i = Item::get_item("lead ball").unwrap();
			i.hidden = true;
			items.add(row, col, i);
		}
	} 

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.333 {
		for _ in 0..rand::thread_rng().gen_range(0, 12) {
			let mut i = Item::get_item("doubloon").unwrap();
			i.hidden = true;
			items.add(row, col, i);
		}
	} 

	if rand::thread_rng().gen_range(0.0, 1.0) < 0.10 {
		let mut i = Item::get_item("rusty cutlass").unwrap();
		i.hidden = true;
		items.add(row, col, i);
	} 
}

fn add_shipwreck(map: &mut Vec<Vec<Tile>>, seacoast: &VecDeque<(usize, usize)>,
			items: &mut ItemsTable,
			world_offset_r: usize,
			world_offset_c: usize,) {
	let loc = rand::thread_rng().gen_range(0, seacoast.len());
	let centre = seacoast[loc];	

	let deck = Tile::Shipwreck(ship::DECK_ANGLE, ship::random_name()); 
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
				add_cache(items, loc_r, loc_c);
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
