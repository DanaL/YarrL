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

use sdl2::pixels::Color;

use crate::display;

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

/*
pub const AFT_NE: char = '\u{25E5}';
pub const AFT_SE: char = '\u{25E2}';
pub const AFT_SW: char = '\u{25E3}';
pub const AFT_NW: char = '\u{25E4}';
pub const AFT_W: char = '\u{25C0}';
pub const AFT_E: char = '\u{25B6}';
pub const AFT_N: char = '\u{25B2}';
pub const AFT_S: char = '\u{25BC}';
*/

#[derive(Debug)]
pub struct Ship {
	pub name: String,
	pub row: usize,
	pub col: usize,
	pub bearing: u8,
	pub anchored: bool,
}

impl Ship {
	pub fn new(name: String) -> Ship {
		Ship { 
			name, 
			row: 0, 
			col: 0, 
			bearing: 0,
			anchored: false,
	 	}
	}

	pub fn random_name() -> String {
		"The Guppy".to_string()
	}
}
