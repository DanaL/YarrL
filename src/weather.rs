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
            for _ in 0..250 {
                let r = rand::thread_rng().gen_range(-s.radius, s.radius) + s.row as i32;
                let c = rand::thread_rng().gen_range(-s.radius, s.radius) + s.col as i32;

                let roll = rand::thread_rng().gen_range(0.0, 1.0);
                if in_bounds(&state.map[&state.map_id], r, c) && roll <= s.intensity {
                    self.clouds.insert((r as usize, c as usize));
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

