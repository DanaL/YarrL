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

use super::{GameState, ItemsTable, Map};
use crate::map::generate_island;
use crate::ship::Ship;
use crate::map;
use crate::map::Tile;

const WORLD_WIDTH: usize = 200;
const WORLD_HEIGHT: usize = 200;

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
	let island = map::generate_island(65);
	for r in 0..island.len() {
		for c in 0..island.len() {
			state.map[r + 5][c + 5] = island[r][c];
		}
	}

	let island = map::generate_island(65);
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
						

