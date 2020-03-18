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

use std::collections::HashSet;
use rand::Rng;

use serde::{Serialize, Deserialize};

use crate::map::in_bounds;
use crate::util::bresenham_circle;
use super::GameState;

// Currently, weather consists only of fog

#[derive(Serialize, Deserialize)]
pub struct Weather {
    pub systems: Vec<WeatherSystem>,
    pub clouds: HashSet<(usize, usize)>,
}

impl Weather {
    pub fn new() -> Weather {
        Weather { systems:Vec::new(), clouds: HashSet::new() }
    }

    // this is 100% temp/prototype code
    pub fn calc_clouds(&mut self, state: &GameState) {
        self.clouds.clear();
    
        for s in &self.systems {
			for r in 1..=s.radius {
				let pts = bresenham_circle(s.row as i32, s.col as i32, r);
				for pt in pts {
					let roll = rand::thread_rng().gen_range(0.0, 1.0);
					if roll < s.intensity && in_bounds(&state.map[&state.map_id], pt.0, pt.1) {
						self.clouds.insert((pt.0 as usize, pt.1 as usize));
					}
				}
			}
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WeatherSystem { 
    row: usize,
    col: usize,
    radius: i32,
    intensity: f32,
}

impl WeatherSystem {
    pub fn new(row: usize, col: usize, radius: i32, intensity: f32) -> WeatherSystem {
        WeatherSystem { row, col, radius, intensity, }
    }
}

