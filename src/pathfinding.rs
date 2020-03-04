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

use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::cmp::Ordering;

use crate::map;
use crate::map::{in_bounds, is_passable, is_passable_by_water};

#[derive(Debug)]
struct ASNode {
	loc: (usize, usize),
	parent: (usize, usize),
	f: usize,
	g: usize,
	h: usize,
}

impl ASNode {
	fn new(loc: (usize, usize), p: (usize, usize), f: usize, g: usize, h: usize) -> ASNode {
		ASNode { loc, parent:p, f,g, h }
	}
}

#[derive(Eq, Debug)]
struct ASQueueItem {
	loc: (usize, usize),
	f: i32,
}

impl ASQueueItem {
	fn new(loc: (usize, usize), f: i32) -> ASQueueItem {
		ASQueueItem { loc, f }
	}
}

impl Ord for ASQueueItem {
	fn cmp(&self, other: &Self) -> Ordering {
        self.f.cmp(&other.f)
    }
}

impl PartialOrd for ASQueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ASQueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f
    }
}

pub fn manhattan_d(ax: usize, ay: usize, bx: usize, by: usize) -> usize {
	((ax as i32 - bx as i32).abs() + (ay as i32 - by as i32).abs()) as usize	
}

fn get_path_from_nodes(nodes: &HashMap<(usize, usize), ASNode>,
		path: &mut Vec<(usize, usize)>,
		sr: usize, sc: usize, er: usize, ec: usize) {
	let mut cr = er;
	let mut cc = ec;

	while cr != sr || cc != sc {
		path.push((cr, cc));
		let n = &nodes[&(cr, cc)];
		cr = n.parent.0;
		cc = n.parent.1;	
	}

	path.push((sr, sc));
	path.reverse();
}

pub fn passable_by_me(tile: map::Tile, valid: &HashSet<map::Tile>) -> bool {
	valid.contains(&tile)
}

// I think I could get rid of the redundant data structures with the use
// of smart pointers (I am keeping a list of visited squares as well as 
// a hash table of square info to avoid having to fight with the borrow
// checker and I shouldn't need both). But that'll be for the post-7DRL 
// future when I have more time.
// I hate this cut and paste but its 7DRL and I've no time to be fancy
pub fn find_path(
		map: &Vec<Vec<map::Tile>>, 
		start_r: usize, start_c: usize, 
		end_r: usize, end_c: usize,
		passable_tiles: &HashSet<map::Tile>) -> Vec<(usize, usize)> {

	if !passable_by_me(map[end_r][end_c], &passable_tiles) {
		// The goal is on an impassable sq so gotta try something else
		return Vec::new();
	}

	let mut nodes = HashMap::new();
	nodes.insert((start_r, start_c), ASNode::new((start_r, start_c), (start_r, start_c), 0, 0, 0));
	let mut open = BinaryHeap::new();
	open.push(ASQueueItem::new((start_r, start_c), 0));

	let mut visited = HashSet::new();
	while open.len() > 0 {
		let current = open.pop().unwrap();
		if current.loc.0 == end_r && current.loc.1 == end_c {
			let mut path = Vec::new();
			get_path_from_nodes(&nodes, &mut path, start_r, start_c, end_r, end_c);
			return path;
		}

		if !visited.contains(&current.loc) {
			visited.insert((current.loc.0, current.loc.1));
		}
		
		for r in -1..2 {
			for c in -1..2 {
				if r == 0 && c == 0 { continue; }
	
				let nr = (current.loc.0 as i32 + r) as usize;
				let nc = (current.loc.1 as i32 + c) as usize;
				// note that at the moment this only considers whether
				// the tile is passable and not say occupied by anotehr 
				// creature
				if !in_bounds(map, nr as i32, nc as i32) {
					continue;
				}
				if !passable_by_me(map[nr][nc], passable_tiles) {
					continue;
				}
	
				let g = nodes[&current.loc].g + 1;
				let h = manhattan_d(nr, nc, end_r, end_c);
				let f = g + h;

				let next = ASNode::new((nr, nc), (current.loc.0, current.loc.1), f, g, h);
				if !visited.contains(&next.loc) {
					open.push(ASQueueItem::new((nr, nc), -(f as i32)));
				}

				if !nodes.contains_key(&next.loc) {
					nodes.insert((nr, nc), next);
				} else if g < nodes[&next.loc].g {
					let n = nodes.get_mut(&next.loc).unwrap();
					n.g = g;
					n.parent = (nr, nc);
				}
			}
		}
	}

	Vec::new()
}
