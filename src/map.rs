extern crate rand;
extern crate sdl2;

use std::collections::HashMap;
use std::collections::HashSet;
use std::f32;

use rand::Rng;
use sdl2::pixels::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tile {
	Blank,
	Wall,
	Tree,
	Dirt,
	Grass,
	Player,
	Water,
	DeepWater,
	Sand,
	Mountain,
	SnowPeak,
	Gate,
	StoneFloor,
	Thing(Color, char), // ie., NPC or item so far
}

// Probably at some point in the dev process, I'll need to begin 
// storing the map in a struct with extra info instead of just
// a matrix of Tiles. Then, I won't have to recalculate height and
// width every time I call the in_bounds() method
pub fn in_bounds(map: &Vec<Vec<Tile>>, r: i32, c: i32) -> bool {
	let height = map.len() as i32;
	let width = map[0].len() as i32;

	r >= 0 && c >= 0 && r < height && c < width
}

pub fn is_clear(tile: Tile) -> bool {
	match tile {
		Tile::Wall | Tile::Blank | Tile::Mountain | Tile::SnowPeak => false,
		_ => true,
	}
}

pub fn is_passable(tile: Tile) -> bool {
	match tile {
		Tile::DeepWater | Tile::Wall | Tile::Blank |
		Tile::Mountain | Tile::SnowPeak | Tile::Gate => false,
		_ => true,
	}
}

fn val_to_terrain(val: f32) -> Tile {
	if val < -0.5 {
		return Tile::DeepWater;
	} else if val < -0.25 {
		return Tile::Water;
	} else if val < 0.20 {
		return Tile::Sand;	
	} else if val < 0.45 {
		return Tile::Grass;
	} else if val < 0.85 {
		return Tile::Tree;
	} else if val < 1.5 {
		return Tile::Mountain;
	}

	Tile::SnowPeak
}

fn fuzz(width: usize, scale: f32) -> f32 {
	(rand::thread_rng().gen_range(0.0, 1.0) * 2f32 - 1f32) * width as f32 * scale	
}

fn diamond_step(grid: &mut Vec<Vec<f32>>, r: usize, c: usize, width: usize, scale: f32) {
	let mut avg = grid[r][c];
	avg += grid[r][c + width - 1];
	avg += grid[r + width - 1][c];
	avg += grid[r + width - 1][c + width - 1];
	avg /= 4f32;

	grid[r + width /2][c + width / 2] = avg + fuzz(width, scale);
}

fn calc_diamond_avg(grid: &mut Vec<Vec<f32>>, r: usize, c: usize, width: usize, scale: f32) {
	let mut count = 0;
	let mut avg = 0.0;
	if width <= c {
		avg += grid[r][c - width];
		count += 1;
	}
	if c + width < grid.len() {
		avg += grid[r][c + width];
		count += 1;
	}
	if width <= r {
		avg += grid[r - width][c];
		count += 1;
	}
	if r + width < grid.len() {
		avg += grid[r + width][c];
		count += 1;
	}
	
	grid[r][c] = avg / count as f32 + fuzz(width, scale);
}

fn square_step(grid: &mut Vec<Vec<f32>>, r: usize, c: usize, width: usize, scale: f32) {
	let half_width = width / 2;

	calc_diamond_avg(grid, r - half_width, c, half_width, scale);
	calc_diamond_avg(grid, r + half_width, c, half_width, scale);
	calc_diamond_avg(grid, r, c - half_width, half_width, scale);
	calc_diamond_avg(grid, r, c + half_width, half_width, scale);
}

fn diamond_sq(grid: &mut Vec<Vec<f32>>, r: usize, c: usize, width: usize, scale: f32) {
	diamond_step(grid, r, c, width, scale);
	let half_width = width / 2;
	square_step(grid, r + half_width, c + half_width, width, scale);

	if half_width == 1 {
		return;
	}

	let new_scale = scale * 1.95;
	diamond_sq(grid, r, c, half_width + 1, new_scale);
	diamond_sq(grid, r, c + half_width, half_width + 1, new_scale);
	diamond_sq(grid, r + half_width, c, half_width + 1, new_scale);
	diamond_sq(grid, r + half_width, c + half_width, half_width + 1, new_scale);
}

fn smooth_map(grid: &mut Vec<Vec<f32>>, width: usize) {
	for r in 0..width {
		for c in 0..width {
			let mut avg = grid[r][c];
			let mut count = 1;

			if r >= 1 {
				if c >= 1 {
					avg += grid[r - 1][c - 1];
					count += 1;
				}
				avg += grid[r - 1][c];
				count += 1;
				if c + 1 < width {
					avg += grid[r - 1][c + 1];
					count += 1;
				}
			}

			if c >= 1 {
				avg += grid[r][c - 1];
				count += 1;
			}
			if c + 1 < width {
				avg += grid[r][c + 1];
				count += 1;
			}

			if r + 1 < width {
				if c >= 1 {
					avg += grid[r + 1][c - 1];
					count += 1;
				}
				avg += grid[r + 1][c];
				count += 1;
				if c + 1 < width {
					avg += grid[r + 1][c + 1];
					count += 1;
				}
			}

			grid[r][c] = avg / count as f32;
		}
	}
}

fn warp_to_island(grid: &mut Vec<Vec<f32>>, width: usize, shift_y: f32) {
	for r in 0..width {
		for c in 0..width {
			let xd = c as f32 / (width as f32 - 1.0) * 2f32 - 1.0;
			let yd = r as f32 / (width as f32 - shift_y) * 2f32 - 1.0;
			let island_size = 0.96;
			grid[r][c] += island_size - f32::sqrt(xd*xd + yd*yd) * 3.0;
		}
	}
}

pub fn generate_island(width: usize) -> Vec<Vec<Tile>> {
	let mut grid = vec![vec![0.0f32; width]; width];

	grid[0][0] = rand::thread_rng().gen_range(0.0, 1.0);
	grid[0][width - 1] = rand::thread_rng().gen_range(0.0, 1.0);
	grid[width - 1][0] = rand::thread_rng().gen_range(0.0, 1.0);
	grid[width - 1][width - 1] = rand::thread_rng().gen_range(0.0, 1.0);

	let initial_scale = 1.0 / width as f32;
	diamond_sq(&mut grid, 0, 0, width, initial_scale);
	smooth_map(&mut grid, width);
	warp_to_island(&mut grid, width, 0.0);

	let mut map: Vec<Vec<Tile>> = Vec::new();
	for r in 0..width {
		let mut row = Vec::new();
		for c in 0..width {
			row.push(val_to_terrain(grid[r][c]));
		}
		map.push(row);
	}

	map
}

fn ds_union(ds: &mut Vec<i32>, r1: i32, r2: i32) {
	let x = ds_find(ds, r1);
	let y = ds_find(ds, r2);

	if x != y {
		ds[y as usize] = x;
	}
}

// It would be smarter to do path compression on find()s
// but I don't think the performance boost is needed here. 
fn ds_find(ds: &Vec<i32>, x: i32) -> i32 {
	if ds[x as usize] < 0 {
		x
	} else {
		ds_find(ds, ds[x as usize])
	}
}

fn find_isolated_caves(grid: &Vec<Vec<bool>>, width: usize, depth: usize) -> Vec<i32> {
	let mut ds: Vec<i32> = vec![-1; width * depth];

	// Run through the grid and union and adjacent floors
	for r in 1..depth - 1 {
		for c in 1..width - 1 {
			if grid[r][c] { continue; }
			let v = (r * width + c) as i32;
		
			if !grid[r - 1][c] { ds_union(&mut ds, v, v - width as i32); }
			if !grid[r + 1][c] { ds_union(&mut ds, v, v + width as i32); }
			if !grid[r][c - 1] { ds_union(&mut ds, v, v - 1); }
			if !grid[r][c + 1] { ds_union(&mut ds, v, v + 1); }
		}
	}

	ds
}

fn find_sets(grid: &Vec<Vec<bool>>, ds: &Vec<i32>, width: usize, depth: usize) -> HashMap<i32, i32> {
	let mut sets: HashMap<i32, i32> = HashMap::new();
	for r in 1..depth - 1 {
		for c in 1..width - 1 {
			if grid[r][c] { continue; }
			let v = (r * width + c) as i32;
			let root = ds_find(ds, v);
			let set = sets.entry(root).or_insert(0);
			*set += 1;
		}
	}

	sets
}

// The caves generated by the cellular automata method can end up disjoint --
// ie., smaller caves separated from each other. First, we need to group the
// floor squares together into sets (or equivalence classes? Is that the term?) 
// using a Disjoint Set ADT.
//
// I'm going to treat squares as adjacent only if they are adjacent along the 
// 4 cardinal compass points.
// 
// To join caves, I look for any wall squares that are separating two different
// caves, then remove them. After that, I'll fill in any smaller caves that are
// still disjoint. (In testing, this still results in decent sized maps. And 
// filling them in means when placing dungeon featuers I can assume any two floor
// squares remaining are accessible from each other.
fn cave_qa(grid: &mut Vec<Vec<bool>>, width: usize, depth: usize) {
	let mut ds = find_isolated_caves(grid, width, depth);

	// Okay, my method to join rooms is to look for single walls that
	// are separating two caves, remove them, and union the two sets.
	// After that I'll fill in any smaller leftover caves
	for r in 1..depth - 1 {
		for c in 1..width - 1 {
			if !grid[r][c] { continue; }
			let i = (r * width + c) as i32;
			let mut adj_sets = HashSet::new();	
			let mut nf = false;
			let mut sf = false;
			let mut ef = false;
			let mut wf = false;

			if !grid[r - 1][c] { 
				adj_sets.insert(ds_find(&ds, i - width as i32));
				nf = true;
			}
						
			if !grid[r + 1][c] { 
				adj_sets.insert(ds_find(&ds, i + width as i32));
				sf = true;
			}

			if !grid[r][c - 1] { 
				adj_sets.insert(ds_find(&ds, i - 1));
				wf = true;
			}

			if !grid[r][c + 1] { 
				adj_sets.insert(ds_find(&ds, i + 1));
				ef = true;
			}

			if adj_sets.len() > 1 {
				grid[r][c] = false;
				if nf { ds_union(&mut ds, i, i - width as i32); }
				if sf { ds_union(&mut ds, i, i + width as i32); }
				if wf { ds_union(&mut ds, i, i - 1); }
				if ef { ds_union(&mut ds, i, i + 1); }
			}
		}
	}

	let sets = find_sets(grid, &mut ds, width, depth);
	let mut largest_set = 0;
	let mut largest_count = 0;
	for s in sets {
		if s.1 > largest_count { 
			largest_set = s.0; 
			largest_count = s.1;
		}
	}

	for r in 1..depth - 1 {
		for c in 1..width - 1 {
			if grid[r][c] { continue; }
			let set = ds_find(&ds, (r * width + c) as i32);
			if set != largest_set {
				grid[r][c] = true;
			}
		}
	}
}

fn count_neighbouring_walls(grid: &Vec<Vec<bool>>, row: i32, col: i32, width: i32, depth: i32) -> u32 {
	let mut adj_walls = 0;

	for r in -1..2 {
		for c in -1..2 {
			let nr = row + r;
			let nc = col + c;
			if nr < 0 || nc < 0 || nr == depth || nc == width {
				adj_walls += 1;
			} else if !(nr == 0 && nc == 0) && grid[nr as usize][nc as usize] {
				adj_walls += 1;
			}
		}
	}	

	adj_walls
}

pub fn generate_cave(width: usize, depth: usize) -> Vec<Vec<Tile>> {
	let mut grid = vec![vec![true; width]; depth];

	// Set some initial squares to be floors (false indidcates floor in our
	// initial grid)
	for r in 0..depth {
		for c in 0..width {
			let x: f64 = rand::thread_rng().gen();
			if x < 0.55 {
				grid[r][c] = false;
			}
		}
	}

	// We are using the 4-5 rule here (if a square has
	// 3 or fewer adjacents walls, it starves and becomes a floor,
	// if it has greater than 5 adj walls, it becomes a wall, otherwise
	// we leave it alone.
	//
	// One generation seems to generate nice enough maps!
	let mut next_gen = vec![vec![false; width]; depth];
	for r in 1..depth - 1 {
		for c in 1..width - 1 {
			let adj_walls = count_neighbouring_walls(&grid, r as i32, c as i32, width as i32, depth as i32);

			if adj_walls < 4 {
				next_gen[r][c] = false;
			} else if adj_walls > 5 {
				next_gen[r][c] = true;
			} else {
				next_gen[r][c] = grid[r][c];
			}
		}
	}

	// set the border
	for c in 0..width {
		next_gen[0][c] = true;
		next_gen[depth - 1][c] = true;	
	}
	for r in 1..depth - 1 {
		next_gen[r][0] = true;
		next_gen[r][width - 1] = true;
	}

	cave_qa(&mut next_gen, width, depth);

	let mut map: Vec<Vec<Tile>> = Vec::new();
	for r in next_gen {
		let mut row = Vec::new();
		for sq in r {
			let tile = if sq {
				Tile::Wall
			} else {
				Tile::StoneFloor
			};
			row.push(tile);
		}
		map.push(row);
	}
	
	map
}
