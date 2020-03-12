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
use std::collections::VecDeque;
use std::cmp::Ordering;

use crate::display::GameUI;
use crate::map;
use crate::map::Tile;
use crate::ship::Ship;
use crate::util::cartesian_d;
use super::GameState;

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

fn backtrace_path(goal_r: usize, goal_c: usize, parents: &HashMap<(usize, usize), (usize, usize)>) ->
			Vec<(usize, usize)> {
	let mut c = (goal_r, goal_c);	
	let mut v = vec![c];
	loop {
		if !parents.contains_key(&c) { break; }
		let p = parents.get(&c).unwrap();
		v.push(*p);
		c = *p;
	}
	
	v.reverse();

	v
}

// If the target location cannot be reached (eg., a shark wants to swim
// toward the player who is standing on a beach), then try to find a nearby
// square to swim to. I am going to floodfill to find all reachable squares
// and return one that is near the player. 
fn find_nearest_reachable(map: &Vec<Vec<map::Tile>>,
		start_r: usize, start_c: usize,
		end_r: usize, end_c: usize,
		passable_tiles: &HashSet<map::Tile>) -> (usize, usize) {

	let mut sqs = BinaryHeap::new();
	let mut visited = HashSet::new();
	let mut queue = VecDeque::new();
	queue.push_back((start_r, start_c));

	while queue.len() > 0 {
		let curr = queue.pop_front().unwrap();
		if visited.contains(&curr) { continue; }
		visited.insert(curr);
		
		let dis_to_goal = cartesian_d(end_r, end_c, curr.0, curr.1) as i32;
		sqs.push(ASQueueItem::new((curr.0, curr.1), -dis_to_goal));

		for r in -1..2 {
			for c in -1..2 {
				if r == 0 && c == 0 { continue; }
	
				let nr = curr.0 as i32 + r;
				let nc = curr.1 as i32 + c;
				if !map::in_bounds(map, nr, nc) { continue; }
				if !passable_by_me(&map[nr as usize][nc as usize], passable_tiles) { continue; }

				let dis_from_start = cartesian_d(start_r, start_c, nr as usize, nc as usize) as i32;
				if dis_from_start > 30 { continue; }
			
				let next_loc = (nr as usize, nc as usize);
				if !visited.contains(&next_loc) { 
					queue.push_back(next_loc);
				}
			}
		}	
	}

	if sqs.len() > 0 {
		let n = sqs.pop().unwrap();
		n.loc
	} else {
		(0, 0)
	}
}

// This is based straight-up on the algorithm description on Wikipedia.
fn astar(
		state: &GameState,
		start_r: usize, start_c: usize, 
		end_r: usize, end_c: usize,
		passable_tiles: &HashSet<map::Tile>,
		ships: &HashMap<(usize, usize), Ship>) -> Vec<(usize, usize)> {
	let mut queue = BinaryHeap::new();
	let mut in_queue = HashSet::new();
	let mut parents = HashMap::new();
	let mut g_scores = HashMap::new();
	g_scores.insert((start_r, start_c), 0);
	let goal = (end_r, end_c);

	queue.push(ASQueueItem::new((start_r, start_c), 0)); 
	in_queue.insert((start_r, start_c));

	while queue.len() > 0 {
		let node = queue.pop().unwrap();
		let curr = node.loc;
		if curr == goal {
			return backtrace_path(end_r, end_c, &parents);
		}

		for r in -1..2 {
			for c in -1..2 {
				if r == 0 && c == 0 { continue; }
				let nr = curr.0 as i32 + r;
				let nc = curr.1 as i32 + c;
				if !map::in_bounds(&state.map, nr, nc) { continue; }

				let n_loc = (nr as usize, nc as usize);
				if !passable_by_me(&state.map[n_loc.0][n_loc.1], passable_tiles) { continue; }
				if n_loc != goal && !super::sq_is_open(state, ships, n_loc.0, n_loc.1) { continue; }

				let tentative_score = *g_scores.get(&curr).unwrap() + 1;
				let mut g = std::u32::MAX;
				if g_scores.contains_key(&n_loc) {
					g = *g_scores.get(&n_loc).unwrap();
				}

				if tentative_score < g {
					g_scores.entry(n_loc)
							.and_modify(|v| { *v = tentative_score } )
							.or_insert(tentative_score);

					let mut d_to_goal = (nr - end_r as i32).abs() + (nc - end_c as i32).abs();
					if d_to_goal < 0 { d_to_goal *= -1 }
					d_to_goal += tentative_score as i32;

					if !in_queue.contains(&n_loc) {
						let p = parents.entry(n_loc).or_insert(curr);
						*p = curr;
						queue.push(ASQueueItem::new(n_loc, -d_to_goal)); 
						in_queue.insert(n_loc);
					}
				}
			}
		}
	}
	
	Vec::new()
}
	
pub fn passable_by_me(tile: &map::Tile, valid: &HashSet<map::Tile>) -> bool {
	valid.contains(&tile)
}

pub fn find_path(
		state: &GameState,
		start_r: usize, start_c: usize, 
		end_r: usize, end_c: usize,
		passable_tiles: &HashSet<map::Tile>,
		ships: &HashMap<(usize, usize), Ship>) -> Vec<(usize, usize)> {

	let mut goal_r = end_r;
	let mut goal_c = end_c;

	// If the target is a square that cannot be stepped on (eg, player on a beach,
	// shark in the water hunting them) we will instead find the nearest reachable 
	// spot and seek a path to that instead.
	//
	// (I could also do this if the astar() returns no path but worry that would 
	// start to get expensive)
	if !passable_by_me(&state.map[end_r][end_c], &passable_tiles) {
		// The goal is on an impassable sq so gotta try something else
		let res = find_nearest_reachable(&state.map, start_r, start_c, end_r, end_c, passable_tiles);
		if res == (0, 0) {
			return Vec::new();
		}

		goal_r = res.0;
		goal_c = res.1;
	}

	astar(state, start_r, start_c, goal_r, goal_c, passable_tiles, ships)
}
