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

use std::collections::HashMap;

use crate::actor::Player;
use crate::display::{WHITE, LIGHT_BLUE, BROWN};
use crate::map;
use crate::map::in_bounds;
use super::{GameState, Map, NPCTable};
use crate::items::{ItemsTable, TileInfo};
use crate::ship;
use crate::ship::Ship;

// I really regret not doing something like in crashRun where instead of 
// just storing a map of tiles/characters, I store objects that can determine
// what tile to show themselves. Looking at separate tile/npc/items/ships
// tables to see what tile to show is so kludgy. The breaking point is ships
// since they cover three tiles. Oh well! Just gotta get 7DRL done!
// (That said, Rust doesn't really have objects which would make the crashRun
// scheme complicated, I think)

fn calc_actual_tile(r: usize, c: usize, map: &Map, 
		npcs: &NPCTable, items: &ItemsTable) -> map::Tile {
	if items.count_at(r, c) > 0 {
		let i = items.peek_top(r, c);
		let ti = i.get_tile_info();
		map::Tile::Thing(ti.0, ti.1)
	} else if npcs.contains_key(&(r, c)) {
		let m = npcs.get(&(r, c)).unwrap();
		map::Tile::Thing(m.color, m.symbol)
	} else {
		map[r][c]
	}
}

// Using bresenham line casting to detect blocked squares. If a ray hits
// a Wall before reaching target then we can't see it. Bresenham isn't 
// really a good way to do this because it leaves blindspots the further
// away you get and also is rather ineffecient (you visit the same squares 
// several times). My original plan, after making a prototype with beamcasting,
// was to switch to shadowcasting. But bresenham seemed sufficiently fast
// and I haven't seen and blindspots (perhaps because I'm keeping the FOV at
// 40x20).
//
// As well, I wanted to have the trees obscure/reduce the FOV instead of outright
// blocking vision and I couldn't think of a simple way to do that with 
// shadowcasting.
fn mark_visible(r1: i32, c1: i32, r2: i32, c2: i32, map: &Map,
		npcs: &NPCTable, items: &ItemsTable,
		v_matrix: &mut Vec<Vec<map::Tile>>) {
	let mut r = r1;
	let mut c = c1;
	let mut error = 0;

	let mut r_step = 1;
	let mut delta_r = r2 - r;
	if delta_r < 0 {
		delta_r = -delta_r;
		r_step = -1;
	} 

	let mut c_step = 1;
	let mut delta_c = c2 - c;
	if delta_c < 0 {
		delta_c = -delta_c;
		c_step = -1;
	} 

	let mut r_end = r2;
	let mut c_end = c2;
	if delta_c <= delta_r {
		let criterion = delta_r / 2;
		loop {
			if r_step > 0 && r >= r_end + r_step {
				break;
			} else if r_step < 0 && r <= r_end + r_step {
				break;
			}

			if !map::in_bounds(map, r, c) {
				return;
			}

			let vm_r = (r - r1 + 10) as usize;
			let vm_c = (c - c1 + 20) as usize;
			v_matrix[vm_r][vm_c] = calc_actual_tile(r as usize, c as usize, map, npcs, items);

			if !map::is_clear(map[r as usize][c as usize]) {
				return;
			}

			// I want trees to not totally block light, but instead reduce visibility
			if map::Tile::Tree == map[r as usize][c as usize] && !(r == r1 && c == c1) {
				if r_step > 0 {
					r_end -= 3;
				} else {
					r_end += 3;
				}
			}

			r += r_step;
			error += delta_c;
			if error > criterion {
				error -= delta_r;
				c += c_step;
			}
		} 	
	} else {
		let criterion = delta_c / 2;
		loop {
			if c_step > 0 && c >= c_end + c_step {
				break;
			} else if c_step < 0 && c <= c_end + c_step {
				break;
			}

			if !map::in_bounds(map, r, c) {
				return;
			}

			let vm_r = (r - r1 + 10) as usize;
			let vm_c = (c - c1 + 20) as usize;
			v_matrix[vm_r][vm_c] = calc_actual_tile(r as usize, c as usize, map, npcs, items);

			if !map::is_clear(map[r as usize][c as usize]) {
				return;
			}
		
			// Same as above, trees partially block vision instead of cutting it off
			// altogether
			if map::Tile::Tree == map[r as usize][c as usize] && !(r == r1 && c == c1) {
				if c_step > 0 {
					c_end -= 3;
				} else {
					c_end += 3;
				}
			}
			
			c += c_step;
			error += delta_r;
			if error > criterion {
				error -= delta_c;
				r += r_step;
			}
		}
	}
}

fn add_ship(v_matrix: &mut Vec<Vec<map::Tile>>, row: usize, col: usize, ship: &Ship) {
	let bearing = ship.bearing;
	v_matrix[row][col] = map::Tile::ShipPart(ship.deck_ch);
	
	let delta_row_bow = ship.bow_row as i8 - ship.row as i8;
	let delta_col_bow = ship.bow_col as i8 - ship.col as i8;
	let delta_row_aft = ship.aft_row as i8 - ship.row as i8;
	let delta_col_aft = ship.aft_col as i8 - ship.col as i8;

	let bow_row = (delta_row_bow + row as i8) as usize;
	let bow_col = (delta_col_bow + col as i8) as usize;
	let aft_row = (delta_row_aft + row as i8) as usize;
	let aft_col = (delta_col_aft + col as i8) as usize;
	
	if in_bounds(v_matrix, bow_row as i32, bow_col as i32) && v_matrix[bow_row][bow_col] != map::Tile::Blank {
		v_matrix[bow_row][bow_col] = map::Tile::ShipPart(ship.bow_ch);
	} 
	if in_bounds(v_matrix, aft_row as i32, aft_col as i32) && v_matrix[aft_row][aft_col] != map::Tile::Blank {
		v_matrix[aft_row][aft_col] = map::Tile::ShipPart(ship.aft_ch);
	} 
}

// Because ships are multi-tile things, it's simpler to just add them to the map later...
fn add_ships_to_v_matrix(
		map: &Vec<Vec<map::Tile>>,
		v_matrix: &mut Vec<Vec<map::Tile>>, 
		ships: &HashMap<(usize, usize), Ship>,
		player_row: usize, player_col: usize, 
		height: usize, width: usize) {
	let half_height = (height / 2) as i32;
	let half_width = (width / 2) as i32;

	for r in -half_height..half_height {
		for c in -half_width..half_width {
			// I'm very in love with how Rust refuses to do any integer casting right now...
			if !in_bounds(map, r + player_row as i32, c + player_col as i32) { continue; }
			let curr_r = (r + player_row as i32) as usize;
			let curr_c = (c + player_col as i32) as usize;
			let loc = ((r + player_row as i32) as usize, (c + player_col as i32) as usize);
			if v_matrix[(r + half_height) as usize][(c + half_width) as usize] != 
					map::Tile::Blank && ships.contains_key(&loc) {
				let ship = ships.get(&loc).unwrap();
				add_ship(v_matrix, (r + half_height) as usize, (c + half_width) as usize, &ship);
			}
		}
	}
}

// not yet taking into account objects on the ground and monsters...
pub fn calc_v_matrix(
		state: &GameState,
		items: &ItemsTable,
		ships: &HashMap<(usize, usize), Ship>,
		player: &Player,
		height: usize, width: usize) -> Vec<Vec<map::Tile>> {
	let mut v_matrix: Vec<Vec<map::Tile>> = Vec::new();
	for _ in 0..height {
		v_matrix.push(vec![map::Tile::Blank; width]);
	}

	let fov_center_r = height / 2;
	let fov_center_c = width / 2;

	for row in 0..height {
		for col in 0..width {
			let offset_r = row as i32 - fov_center_r as i32;
			let offset_c = col as i32 - fov_center_c as i32;
			let actual_r: i32 = player.row as i32 + offset_r;
			let actual_c: i32 = player.col as i32 + offset_c;

			mark_visible(player.row as i32, player.col as i32,
				actual_r as i32, actual_c as i32, state.map, &state.npcs, items, &mut v_matrix);
		}
	}

	add_ships_to_v_matrix(state.map, &mut v_matrix, ships, player.row, player.col, height, width);

	if player.on_ship {
		v_matrix[fov_center_r][fov_center_c] = map::Tile::Player(BROWN);
	} else if state.map[player.row][player.col] == map::Tile::DeepWater {
		v_matrix[fov_center_r][fov_center_c] = map::Tile::Player(LIGHT_BLUE);
	} else {
		v_matrix[fov_center_r][fov_center_c] = map::Tile::Player(WHITE);
	}

	v_matrix
}
