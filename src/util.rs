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


// Some miscellaneous strucs and functions used in a few plces

use std::f32;
use std::fs;

use crate::dice::roll;

#[derive(Debug)]
pub struct NameSeeds {
	pub adjectives: Vec<String>,
	pub nouns: Vec<String>,
	pub proper_nouns: Vec<String>,
}

impl NameSeeds {
	fn new() -> NameSeeds {
		NameSeeds { adjectives: Vec::new(), nouns: Vec::new(), 
			proper_nouns: Vec::new() }
	}
}

pub fn read_names_file() -> NameSeeds {
	let mut ns = NameSeeds::new();

	let contents = fs::read_to_string("names.txt")
        .expect("Unable to find names file!"); 	// I should probably shoot a warning and 
												// a return a small default version of NS
	
	let mut reading = 0;
	for line in contents.split('\n') {
		if line == "" {
			continue;
		} if line == "# Adjectives" {
			reading = 0;
		} else if line == "# Nouns" {
			reading = 1;
		} else if line == "# Proper Nouns" {
			reading = 2;	
		} else {
			if reading == 0 { ns.adjectives.push(line.trim().to_string()); }
			else if reading == 1 { ns.nouns.push(line.trim().to_string()); }
			else if reading == 2 { ns.proper_nouns.push(line.trim().to_string()); }
		}
	}

	ns
}

pub fn capitalize_word(word: &str) -> String {
	// Rust is so intuitive...
	let mut c = word.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn rnd_adj() -> (i32, i32) {
	let x = roll(8, 1, 0);
	if x == 1 { return (-1, -1); }
	else if x == 2 { return (-1, 0); }
	else if x == 3 { return (-1, 1); }
	else if x == 4 { return (0, -1); }
	else if x == 5 { return (0, 1); }
	else if x == 6 { return (1, -1); }
	else if x == 7 { return (1, 0); }
	else { return (1, 1); }
}

pub fn sqs_adj(r0: usize, c0: usize, r1: usize, c1: usize) -> bool {
	let x0 = r0 as i32;
	let y0 = c0 as i32;
	let x1 = r1 as i32;
	let y1 = c1 as i32;

	if x0 - 1 == x1 && y0 - 1 == y1 { return true; } 
	if x0 - 1 == x1 && y0 == y1 { return true; } 
	if x0 - 1 == x1 && y0 + 1 == y1 { return true; } 
	if x0 == x1 && y0 - 1 == y1 { return true; } 
	if x0 == x1 && y0 + 1 == y1 { return true; } 
	if x0 + 1 == x1 && y0 - 1 == y1 { return true; } 
	if x0 + 1 == x1 && y0 == y1 { return true; } 
	if x0 + 1 == x1 && y0 + 1 == y1 { return true; } 

	false
}

pub fn manhattan_d(ax: usize, ay: usize, bx: usize, by: usize) -> usize {
	((ax as i32 - bx as i32).abs() + (ay as i32 - by as i32).abs()) as usize	
}

pub fn cartesian_d(r0: i32, c0: i32, r1: i32, c1: i32) -> usize {
	let v = (r0 - r1) * (r0 - r1) + (c0 - c1) * (c0 - c1);
	let x = f32::sqrt(v as f32);	
	
	x as usize
}

