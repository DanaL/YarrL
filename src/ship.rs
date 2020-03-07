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

use rand::Rng;

use crate::dice;
use crate::util;
use crate::util::capitalize_word;
use crate::util::NameSeeds;

pub const DECK_STRAIGHT: char = '\u{25A0}'; 
pub const DECK_ANGLE: char = '\u{25C6}'; 
pub const BOW_NE: char = '\u{25E5}';
pub const BOW_SE: char = '\u{25E2}';
pub const BOW_SW: char = '\u{25E3}';
pub const BOW_NW: char = '\u{25E4}';
pub const BOW_W: char = '\u{25C0}';
pub const BOW_E: char = '\u{25B6}';
pub const BOW_N: char = '\u{25B2}';
pub const BOW_S: char = '\u{25BC}';
pub const AFT_STRAIGHT: char = '\u{25A0}'; 
pub const AFT_ANGLE: char = '\u{25C6}'; 

#[derive(Debug)]
pub struct Ship {
	pub name: String,
	pub row: usize,
	pub col: usize,
	pub bow_row: usize,
	pub bow_col: usize,
	pub aft_row: usize,
	pub aft_col: usize,
	pub bow_ch: char,
	pub aft_ch: char,
	pub deck_ch: char,
	pub wheel: i8,
	pub bearing: u8,
	pub anchored: bool,
	pub prev_move: (i8, i8),
}

impl Ship {
	pub fn new(name: String) -> Ship {
		Ship { 
			name, 
			row: 0, 
			col: 0, 
			bow_row: 0,
			bow_col: 0,
			aft_row: 0,
			aft_col: 0,
			bow_ch: '\0',
			aft_ch: '\0',
			deck_ch: '\0',
			wheel: 0,
			bearing: 0,
			anchored: true,
			prev_move: (0, 0),
	 	}
	}

	pub fn update_loc_info(&mut self) {
		let boat_tiles: (char, i8, i8, char, i8, i8, char);
		if self.bearing == 0 || self.bearing == 1 || self.bearing == 15 { 
			boat_tiles = (BOW_N, -1, 0, AFT_STRAIGHT, 1, 0, DECK_STRAIGHT);
		} else if self.bearing == 2 {
			boat_tiles = (BOW_NE, -1, 1, AFT_ANGLE, 1, -1, DECK_ANGLE);
		} else if self.bearing == 4 || self.bearing == 5 || self.bearing == 3 {
			boat_tiles = (BOW_E, 0, 1, AFT_STRAIGHT, 0, -1, DECK_STRAIGHT);
		} else if self.bearing == 6 {
			boat_tiles = (BOW_SE, 1, 1, AFT_ANGLE, -1, -1, DECK_ANGLE);
		} else if self.bearing == 7 || self.bearing == 8 || self.bearing == 9 {
			boat_tiles = (BOW_S, 1, 0, AFT_STRAIGHT, -1, 0, DECK_STRAIGHT);
		} else if self.bearing == 10 {
			boat_tiles = (BOW_SW, 1, -1, AFT_ANGLE, -1, 1, DECK_ANGLE);
		} else if self.bearing == 11 || self.bearing == 12 || self.bearing == 13 {
			boat_tiles = (BOW_W, 0, -1, AFT_STRAIGHT, 0, 1, DECK_STRAIGHT);
		} else {
			boat_tiles = (BOW_NW, -1, -1, AFT_ANGLE, 1, 1, DECK_ANGLE);
		}

		self.bow_ch = boat_tiles.0;
		self.bow_row = ((self.row as i32) + boat_tiles.1 as i32) as usize;
		self.bow_col = ((self.col as i32) + boat_tiles.2 as i32) as usize;
		self.aft_ch = boat_tiles.3;
		self.aft_row = ((self.row as i32) + boat_tiles.4 as i32) as usize;
		self.aft_col = ((self.col as i32) + boat_tiles.5 as i32) as usize;
		self.deck_ch = boat_tiles.6;
	}
}

pub fn random_name(allow_ys: bool) -> String {
	let mut name = String::from("");
	let ns = util::read_names_file();
	
	// not every ship gets to be part of the Royal Yendorian Navy!
	if allow_ys && dice::roll(7, 1, 0) == 1 {
		name.push_str("Y.S. "); 
	}

	let r = rand::thread_rng().gen_range(0, ns.adjectives.len());
	let adj = &ns.adjectives[r];

	let r = rand::thread_rng().gen_range(0, ns.nouns.len());
	let mut noun = &ns.nouns[r];

	loop {
		// Veto-ing this one. I imagine in the future I'll probably
		// find more cross combos
		if !(adj == "flirty" && noun == "child") { break }

		let r = rand::thread_rng().gen_range(0, ns.nouns.len());
		noun = &ns.nouns[r];
	}

	if dice::roll(10, 1, 0) < 10 {
		name.push_str(&capitalize_word(adj));
		name.push(' ');
	}

	name.push_str(&capitalize_word(noun));

	name
}
