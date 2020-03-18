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

use std::collections::{HashMap, HashSet};

use crate::actor::NPCTracker;
use crate::display::{WHITE, LIGHT_BLUE, BROWN};
use crate::map;
use super::{GameState, Map};
use crate::items::{ItemsTable, TileInfo};
use crate::ship::Ship;
use crate::util;
use crate::weather::Weather;
use super::{FOV_WIDTH, FOV_HEIGHT};

// Kind of ugly by why recalculate these everytime?
#[inline]
fn radius_3() -> Vec<(i32, i32)> {
	let c = vec![(3, 0), (3, 0), (-3, 0), (-3, 0), (0, 3), (0, -3), (0, 3), (0, -3), (3, 1), (3, -1), 
		(-3, 1), (-3, -1), (1, 3), (1, -3), (-1, 3), (-1, -3), (2, 2), (2, -2), (-2, 2), (-2, -2), 
		(2, 2), (2, -2), (-2, 2), (-2, -2)];
	c	
}

#[inline]
fn radius_5() -> Vec<(i32, i32)> {
	let c = vec![(5, 0), (5, 0), (-5, 0), (-5, 0), (0, 5), (0, -5), (0, 5), (0, -5), (5, 1), (5, -1), 
		(-5, 1), (-5, -1), (1, 5), (1, -5), (-1, 5), (-1, -5), (5, 2), (5, -2), (-5, 2), (-5, -2), (2, 5), 
		(2, -5), (-2, 5), (-2, -5), (4, 3), (4, -3), (-4, 3), (-4, -3), (3, 4), (3, -4), (-3, 4), (-3, -4),
		(-3, -3), (3, 3), (-3, 3), (3, -3)];

	c	
}

#[inline]
fn radius_7() -> Vec<(i32, i32)> {
	let c = vec![(7, 0), (7, 0), (-7, 0), (-7, 0), (0, 7), (0, -7), (0, 7), (0, -7), (7, 1), (7, -1), (-7, 1), 
		(-7, -1), (1, 7), (1, -7), (-1, 7), (-1, -7), (7, 2), (7, -2), (-7, 2), (-7, -2), (2, 7), (2, -7), 
		(-2, 7), (-2, -7), (6, 3), (6, -3), (-6, 3), (-6, -3), (3, 6), (3, -6), (-3, 6), (-3, -6), (6, 4), 
		(6, -4), (-6, 4), (-6, -4), (4, 6), (4, -6), (-4, 6), (-4, -6), (5, 5), (5, -5), (-5, 5), (-5, -5), 
		(5, 5), (5, -5), (-5, 5), (-5, -5), (-4, -5), (4, 5), (-4, 5), (4, -5), (-5, -4), (5, 4), (-5, 4),
		(5, -4)];

	c
}

fn radius_9() -> Vec<(i32, i32)> {
	let c = vec![(9, 0), (9, 0), (-9, 0), (-9, 0), (0, 9), (0, -9), (0, 9), (0, -9), (9, 1), (9, -1), (-9, 1), 
		(-9, -1), (1, 9), (1, -9), (-1, 9), (-1, -9), (9, 2), (9, -2), (-9, 2), (-9, -2), (2, 9), (2, -9), 
		(-2, 9), (-2, -9), (9, 3), (9, -3), (-9, 3), (-9, -3), (3, 9), (3, -9), (-3, 9), (-3, -9), (8, 4), 
		(8, -4), (-8, 4), (-8, -4), (4, 8), (4, -8), (-4, 8), (-4, -8), (8, 5), (8, -5), (-8, 5), (-8, -5), 
		(5, 8), (5, -8), (-5, 8), (-5, -8), (7, 6), (7, -6), (-7, 6), (-7, -6), (6, 7), (6, -7), (-6, 7), 
		(-6, -7), (-6, -6), (6, 6), (6, -6), (-6, 6), (-7, -5), (7, 5), (-7, 5), (7, -5), (-5, -7), (5, 7),
		(-5, 7), (5, -7)];

	c
}

#[inline]
fn radius_full() -> Vec<(i32, i32)> {
	let mut c = Vec::new();
	let width_radius = (FOV_WIDTH / 2) as i32;
	let height_radius = (FOV_HEIGHT / 2) as i32;

	for col in -width_radius..width_radius {
		c.push((-height_radius, col));
		c.push((height_radius, col));
	}

	for row in -height_radius..height_radius {
		c.push((row, -width_radius));
		c.push((row, width_radius));
	}

	c.push((height_radius, width_radius));

	c	
}

// I really regret not doing something like in crashRun where instead of 
// just storing a map of tiles/characters, I store objects that can determine
// what tile to show themselves. Looking at separate tile/npc/items/ships
// tables to see what tile to show is so kludgy. The breaking point is ships
// since they cover three tiles. Oh well! Just gotta get 7DRL done!
// (That said, Rust doesn't really have objects which would make the crashRun
// scheme complicated, I think)
fn calc_actual_tile(r: usize, c: usize, map: &Map, 
		npcs: &NPCTracker, items: &ItemsTable, weather: &Weather,
            no_fog: &HashSet<(usize, usize)>) -> map::Tile {

    if weather.clouds.contains(&(r, c)) && !no_fog.contains(&(r, c)) {
        map::Tile::Fog
    } else if npcs.is_npc_at(r, c) {
		let ti = npcs.tile_info(r, c);
		map::Tile::Creature(ti.1, ti.0)
	} else if items.count_at(r, c) > 0 {
		let i = items.peek_top(r, c);
		if !i.hidden {
			let ti = i.get_tile_info();
			map::Tile::Thing(ti.0, ti.1)
		} else {
			map[r][c].clone()
		}
	} else {
		map[r][c].clone()
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
fn mark_visible(r1: i32, c1: i32, r2: i32, c2: i32, 
		state: &mut GameState, 
		v_matrix: &mut Vec<bool>, 
        width: usize,
        no_fog: &HashSet<(usize, usize)>) {
	let curr_map = &state.map[&state.map_id];
    let curr_weather = &state.weather[&state.map_id];

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

			if !map::in_bounds(curr_map, r, c) {
				return;
			}

			let vm_r = r - r1 + 10;
			let vm_c = c - c1 + 20;
            let vmi = (vm_r * width as i32 + vm_c) as usize;
			v_matrix[vmi] = true;
			state.world_seen.insert((r as usize, c as usize));

			if !map::is_clear(&curr_map[r as usize][c as usize]) {
				return;
			}

			// I want trees to not totally block light, but instead reduce visibility, but fog 
            // completely blocks light.
            if curr_weather.clouds.contains(&(r as usize, c as usize)) && !no_fog.contains(&(r as usize, c as usize)) {
                return;
            } else if map::Tile::Tree == curr_map[r as usize][c as usize] && !(r == r1 && c == c1) {
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

			if !map::in_bounds(curr_map, r, c) {
				return;
			}

			let vm_r = r - r1 + 10;
			let vm_c = c - c1 + 20;
            let vmi = (vm_r * width as i32 + vm_c) as usize;
			v_matrix[vmi] = true;
			state.world_seen.insert((r as usize, c as usize));

			if !map::is_clear(&curr_map[r as usize][c as usize]) {
				return;
			}
		
			// Same as above, trees partially block vision instead of cutting it off
            if curr_weather.clouds.contains(&(r as usize, c as usize)) && !no_fog.contains(&(r as usize, c as usize)) {
                return;
            } else if map::Tile::Tree == curr_map[r as usize][c as usize] && !(r == r1 && c == c1) {
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

fn add_ship(v_matrix: &mut Vec<map::Tile>, 
            row: usize, 
            col: usize, 
            ship: &Ship,
            width: usize) {
	v_matrix[row * width + col] = map::Tile::ShipPart(ship.deck_ch);
	
	let delta_row_bow = ship.bow_row as i32 - ship.row as i32;
	let delta_col_bow = ship.bow_col as i32 - ship.col as i32;
	let delta_row_aft = ship.aft_row as i32 - ship.row as i32;
	let delta_col_aft = ship.aft_col as i32 - ship.col as i32;

	let bow_row = delta_row_bow + row as i32;
	let bow_col = delta_col_bow + col as i32;
	let aft_row = delta_row_aft + row as i32;
	let aft_col = delta_col_aft + col as i32;
    
    let bow_i = bow_row * width as i32 + bow_col;
    let aft_i = aft_row * width as i32 + aft_col;
    let v_len = v_matrix.len() as i32; 
       
	/* Ship characters will cover terrain and items but not creatures */ 
	if bow_i > 0 && bow_i < v_len { 
		match v_matrix[bow_i as usize] {
			map::Tile::Blank | map::Tile::Creature(_, _) => { /* do nothing */ },
			_ => { v_matrix[bow_i as usize] = map::Tile::ShipPart(ship.bow_ch); },
		}
	} 
	if aft_i > 0 && aft_i < v_len {
		match v_matrix[aft_i as usize] {
			map::Tile::Blank | map::Tile::Creature(_, _) => { /* do nothing */ },
			_ => { v_matrix[aft_i as usize] = map::Tile::ShipPart(ship.aft_ch); },
		}
	} 
}

// Because ships are multi-tile things, it's simpler to just add them to the map later...
fn add_ships_to_v_matrix(
		map: &Vec<Vec<map::Tile>>,
		v_matrix: &mut Vec<map::Tile>, 
		ships: &HashMap<(usize, usize), Ship>,
		player_row: usize, player_col: usize, 
		height: usize, width: usize) {
	let half_height = (height / 2) as i32;
	let half_width = (width / 2) as i32;

	for r in -half_height..half_height {
		for c in -half_width..half_width {
			// I'm very in love with how Rust refuses to do any integer casting right now...
			if !map::in_bounds(map, r + player_row as i32, c + player_col as i32) { continue; }
			let loc = ((r + player_row as i32) as usize, (c + player_col as i32) as usize);
            let i = ((r + half_height) * width as i32 + c + half_width) as usize;
			if v_matrix[i] != map::Tile::Blank && ships.contains_key(&loc) {
				let ship = ships.get(&loc).unwrap();
				add_ship(v_matrix, (r + half_height) as usize, (c + half_width) as usize, &ship, width);
			}
		}
	}
}

pub fn calc_v_matrix(
		state: &mut GameState,
		items: &ItemsTable,
		ships: &HashMap<(usize, usize), Ship>,
		height: usize, width: usize) -> Vec<map::Tile> {
    let size = height * width;
    let mut visible = vec![false; size];
	let fov_center_r = height / 2;
	let fov_center_c = width / 2;

	let perimeter = if state.vision_radius == 3 {
		radius_3()
	} else if state.vision_radius == 5 {
		radius_5()
	} else if state.vision_radius == 7 {
		radius_7()
	} else if state.vision_radius == 9 {
		radius_9()
	} else {
		radius_full()
	};

    let mut no_fog = HashSet::new();
    no_fog.insert((state.player.row - 1, state.player.col - 1));
    no_fog.insert((state.player.row - 1, state.player.col));
    no_fog.insert((state.player.row - 1, state.player.col + 1));
    no_fog.insert((state.player.row, state.player.col - 1));
    no_fog.insert((state.player.row, state.player.col));
    no_fog.insert((state.player.row, state.player.col + 1));
    no_fog.insert((state.player.row + 1, state.player.col - 1));
    no_fog.insert((state.player.row + 1, state.player.col));
    no_fog.insert((state.player.row + 1, state.player.col + 1));
    if state.player.inventory.active_light_source() {
        let pts = util::bresenham_circle(state.player.row as i32, state.player.col as i32, 2);
        for pt in pts {
            no_fog.insert((pt.0 as usize, pt.1 as usize));
        }
    }
    
    let pr = state.player.row as i32;
    let pc = state.player.col as i32;
	// Beamcast to all the points around the perimiter of the viewing
	// area. For YarrL's fixed size FOV this seems to work just fine
	// and cuts about a whole bunch of redundant looping and beam
	// casting.
	for loc in perimeter {
		let actual_r = pr + loc.0;
		let actual_c = pc + loc.1;

		mark_visible(pr, pc, actual_r as i32, actual_c as i32, state, &mut visible, width, &no_fog);
	}

    // Now we know which locations are actually visible from the player's loc, 
    // figure out what tile should be shown. no_fog is a set of squares to ignore
    // fog in. (To make it slightly more difficult for the player to blunder into
    // lava and so they can see neighbouring enemies)
    let mut v_matrix = vec![map::Tile::Blank; size];
	let curr_map = &state.map[&state.map_id];
    for r in 0..height {
        for c in 0..width {
            let j = r * width + c;
            if visible[j] {
                let row = pr - fov_center_r as i32 + r as i32;
                let col = pc - fov_center_c as i32 + c as i32;
                if map::in_bounds(&state.map[&state.map_id], row as i32, col as i32) {
                    v_matrix[j] = calc_actual_tile(row as usize, col as usize, 
                                                   &state.map[&state.map_id], 
                                                   &state.npcs[&state.map_id], 
                                                   items, 
                                                   &state.weather[&state.map_id],
                                                   &no_fog);
                }
            }
        }
    }

	add_ships_to_v_matrix(curr_map, &mut v_matrix, ships, 
			state.player.row, state.player.col, height, width);

    let fov_center_i = fov_center_r * width + fov_center_c;
	if state.player.on_ship {
		v_matrix[fov_center_i] = map::Tile::Player(BROWN);
	} else if curr_map[state.player.row][state.player.col] == map::Tile::DeepWater
			&& !ships.contains_key(&(state.player.row, state.player.col)) {
		v_matrix[fov_center_i] = map::Tile::Player(LIGHT_BLUE);
	} else {
		v_matrix[fov_center_i] = map::Tile::Player(WHITE);
	}

	v_matrix
}
