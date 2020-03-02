use crate::map;
use super::{Map, NPCTable};
use crate::items::{ItemsTable, TileInfo};

fn calc_actual_tile(r: usize, c: usize, map: &Map, 
		npcs: &NPCTable, items: &ItemsTable) -> map::Tile {
	if items.count_at(r, c) > 0 {
		let i = items.peek_top(r, c);
		let ti = i.get_tile_info();
		map::Tile::Thing(ti.0, ti.1)
	} else if npcs.contains_key(&(r, c)) {
		let m = npcs.get(&(r, c)).unwrap().borrow();
		let ti = m.get_tile_info();
		map::Tile::Thing(ti.0, ti.1)
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

// not yet taking into account objects on the ground and monsters...
pub fn calc_v_matrix(
		map: &Vec<Vec<map::Tile>>,
		npcs: &NPCTable,
		items: &ItemsTable,
		player_row: usize, player_col: usize,
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
			let actual_r: i32 = player_row as i32 + offset_r;
			let actual_c: i32 = player_col as i32 + offset_c;

			mark_visible(player_row as i32, player_col as i32,
				actual_r as i32, actual_c as i32, map, npcs, items, &mut v_matrix);
		}
	}
	
	v_matrix[fov_center_r][fov_center_c] = map::Tile::Player;

	v_matrix
}
