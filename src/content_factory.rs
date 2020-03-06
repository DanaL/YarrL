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
use crate::map;
use crate::ship::Ship;
use crate::map::Tile;

pub const WORLD_WIDTH: usize = 200;
pub const WORLD_HEIGHT: usize = 200;

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

	let island = generate_volcanic_island();
	let nw = find_nearest_clear_nw(&island);
	find_hidden_valleys(&island);
	for r in nw.0..island.len() {
		for c in nw.1..island.len() {
			state.map[r + 5][c + 5] = island[r][c];
		}
	}

	let island = map::generate_std_island();
	for r in 0..island.len() {
		for c in 0..island.len() {
			state.map[r + 5][c + 125] = island[r][c];
		}
	}

	// place the player
	state.player.on_ship = true;
	state.player.bearing = 6;
	state.player.wheel = 0;
	state.player.row = 5;
	state.player.col = 5;

	let mut ship = Ship::new("The Minnow".to_string());
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
		snowpeaks = largest_contiguous_block(&island, Tile::SnowPeak);
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

				let tile = map[nr][nc];
				if tile != Tile::Tree && tile != Tile::Mountain && tile != Tile::SnowPeak {
					return HashSet::new();
				}

				if tile == Tile::Tree && !valley.contains(&(nr, nc)) {
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

fn flood_fill_search(map: &Vec<Vec<Tile>>, target: Tile, r: usize, c: usize) 
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

				if map[nr][nc] != target || block.contains(&(nr, nc)) {
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
fn largest_contiguous_block(map: &Vec<Vec<Tile>>, target: Tile) -> HashSet<(usize, usize)> {
	let mut targets_found: HashSet<(usize, usize)> = HashSet::new();
	let mut best = HashSet::new();

	'fuck: for r in 0..map.len() {
		for c in 0..map.len() {
			if map[r][c] == target {
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
