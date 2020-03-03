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

// This file is just a convenient wrapper around rand

extern crate rand;

use rand::Rng;

pub fn roll(faces: u8, dice: u8, modifier: i8) -> u8 {
	let mut sum: i8 = 0;

	for _ in 0..dice {
		let val = rand::thread_rng().gen_range(0.0, 1.0) * faces as f32;
		sum += val as i8 + 1;
	}

	// Whoops gotta fix this because at could end up with u8 underflow here	
	(sum + modifier) as u8
}
