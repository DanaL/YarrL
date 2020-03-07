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

extern crate sdl2;

use std::collections::{HashMap, HashSet, VecDeque};

use crate::content_factory::{WORLD_WIDTH, WORLD_HEIGHT};
use crate::items::Item;
use crate::map;
use super::{Cmd, GameState, Map, FOV_WIDTH, FOV_HEIGHT};

use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Mod;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::surface::Surface;
use sdl2::ttf::Font;
use sdl2::pixels::Color;

pub static BLACK: Color = Color::RGBA(0, 0, 0, 255);
pub static WHITE: Color = Color::RGBA(255, 255, 255, 255);
pub static GREY: Color = Color::RGBA(136, 136, 136, 255);
pub static GREEN: Color = Color::RGBA(144, 238, 144, 255);
pub static DARK_GREEN: Color = Color::RGBA(46, 139, 87, 255);
pub static BROWN: Color = Color::RGBA(150, 75, 0, 255);
pub static DARK_BROWN: Color = Color::RGBA(101, 67, 33, 255);
pub static BLUE: Color = Color::RGBA(0, 0, 200, 255);
pub static LIGHT_BLUE: Color = Color::RGBA(55, 198, 255, 255);
pub static BEIGE: Color = Color::RGBA(255, 178, 127, 255);
pub static BRIGHT_RED: Color = Color::RGBA(208, 28, 31, 255);
pub static GOLD: Color = Color::RGBA(255, 215, 0, 255);
pub static YELLOW: Color = Color::RGBA(255, 225, 53, 255);

const SCREEN_WIDTH: u32 = 58;
const SCREEN_HEIGHT: u32 = 22;
const BACKSPACE_CH: char = '\u{0008}';

#[derive(Debug)]
pub struct SidebarInfo {
	name: String,
	ac: u8,
	curr_hp: u8,
	max_hp: u8,
	wheel: i8,
	bearing: i8,
	turn: u32,
}

impl SidebarInfo {
	pub fn new(name: String, ac: u8, curr_hp: u8, max_hp: u8, 
			wheel: i8, bearing: i8, turn: u32) -> SidebarInfo {
		SidebarInfo { name, ac, curr_hp, max_hp, wheel, bearing, turn }
	}
}

// I have literally zero clue why Rust wants two lifetime parameters
// here for the Font ref but this shuts the compiler the hell up...
pub struct GameUI<'a, 'b> {
	screen_width_px: u32,
	screen_height_px: u32,
	font_width: u32,
	font_height: u32,
	font: &'a Font<'a, 'b>,
	sm_font_width: u32,
	sm_font_height: u32,
	sm_font: &'a Font<'a, 'b>,
	canvas: WindowCanvas,
	event_pump: EventPump,
	pub v_matrix: Map,
	surface_cache: HashMap<(char, Color), Surface<'a>>,
}

impl<'a, 'b> GameUI<'a, 'b> {
	pub fn init(font: &'b Font, sm_font: &'b Font) -> Result<GameUI<'a, 'b>, String> {
		let (font_width, font_height) = font.size_of_char(' ').unwrap();
		let screen_width_px = SCREEN_WIDTH * font_width;
		let screen_height_px = SCREEN_HEIGHT * font_height;

		let (sm_font_width, sm_font_height) = sm_font.size_of_char(' ').unwrap();

		let sdl_context = sdl2::init()?;
		let video_subsystem = sdl_context.video()?;
		let window = video_subsystem.window("YarrL", screen_width_px, screen_height_px)
			.position_centered()
			.opengl()
			.build()
			.map_err(|e| e.to_string())?;

		let v_matrix = vec![vec![map::Tile::Blank; FOV_WIDTH]; FOV_HEIGHT];
		let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
		let gui = GameUI { 
			screen_width_px, screen_height_px, 
			font, font_width, font_height, 
			canvas,
			event_pump: sdl_context.event_pump().unwrap(),
			sm_font, sm_font_width, sm_font_height,
			v_matrix,
			surface_cache: HashMap::new(),
		};

		Ok(gui)
	}

	// I need to handle quitting the app actions here too
	fn wait_for_key_input(&mut self) -> Option<char> {
		loop {
			for event in self.event_pump.poll_iter() {
				match event {
					Event::TextInput { text:val, .. } => { 
						let ch = val.as_bytes()[0];
						return Some(ch as char);
					},
					Event::KeyDown {keycode: Some(Keycode::Return), .. } => return Some('\n'),
					Event::KeyDown {keycode: Some(Keycode::Backspace), .. } => return Some(BACKSPACE_CH),
					Event::KeyDown {keycode: Some(Keycode::Escape), .. } => return None,
					_ => { continue; }
				}
			}
		}
	}

	pub fn show_treasure_map(&mut self, state: &GameState, map: &Item) {
		self.canvas.clear();

		let title = "~Scrawled on a scrap of paper~";
		let mut line = String::from("");
		let padding = (SCREEN_WIDTH as usize / 2 - title.len() / 2) as usize;
		for _ in 0..padding {
			line.push(' ');
		}
		line.push_str(title);
		self.write_line(0, &line, false);

		// Maps are all 15x15 and we want to draw them in the centre of the
		// screen
		let screen_col = SCREEN_WIDTH / 2 - 7;
		for r in 0..25 {
			for c in 0..30 {
				let loc_r = map.nw_corner.0 + r;
				let loc_c = map.nw_corner.1 + c;
				let actual_c = screen_col as usize + c;
				if loc_r == map.x_coord.0 && loc_c == map.x_coord.1 {
					self.write_map_sq(1 + r, actual_c, ('X', BLACK));
				} else {
					let tile = &state.map[loc_r][loc_c];
					let (mut ch, _) = self.sq_info_for_tile(tile);
					if ch == '}' {
						ch = ' ';	
					}
					self.write_map_sq(1 + r, actual_c, (ch, BRIGHT_RED));
				}
			}
		}
		
		self.canvas.present();
		self.wait_for_key_input();
	}

	pub fn show_world_map(&mut self, state: &GameState) {
		self.canvas.clear();

		let title = "~Ye Olde World Map~";
		let mut line = String::from("");
		let padding = (SCREEN_WIDTH as usize / 2 - title.len() / 2) as usize;
		for _ in 0..padding {
			line.push(' ');
		}
		line.push_str(title);
		self.write_line(0, &line, false);

		for sq in state.world_seen.iter() {
			let (_, color) = self.sq_info_for_tile(&state.map[sq.0][sq.1]);
			
			self.canvas.set_draw_color(color);
			self.canvas.fill_rect(Rect::new(sq.1 as i32 * 3, 
					(self.font_height + sq.0 as u32) as i32 * 3, 3, 3));		
		}

		self.canvas.present();
		self.wait_for_key_input();
	}

	pub fn query_single_response(&mut self, question: &str, sbi: &SidebarInfo) -> Option<char> {
		let mut m = VecDeque::new();
		m.push_front(question.to_string());
		self.write_screen(&mut m, sbi);

		self.wait_for_key_input()
	}

	pub fn query_yes_no(&mut self, question: &str, sbi:&SidebarInfo) -> char {
		loop {
			match self.query_single_response(question, sbi) {
				Some('y') => { return 'y'; },
				Some('n') | None => { return 'n'; },
				Some(_) => { continue; },
			}
		}
	}

	pub fn pick_direction(&mut self, sbi: &SidebarInfo) -> Option<(i32, i32)> {
		let mut m = VecDeque::new();
		m.push_front(String::from("In which direction?"));
		self.write_screen(&mut m, sbi);

		loop {
			match self.wait_for_key_input() {
				Some('h') => { return Some((0, -1)); },
				Some('j') => { return Some((1, 0)); },
				Some('k') => { return Some((-1, 0)); },
				Some('l') => { return Some((0, 1)); },
				Some('y') => { return Some((-1, -1)); },
				Some('u') => { return Some((-1, 1)); },
				Some('b') => { return Some((1, -1)); },
				Some('n') => { return Some((1, 1)); },
				Some(_) => { continue; },
				None => { return None; },
			}
		}
	}

	pub fn query_natural_num(&mut self, query: &str, sbi: &SidebarInfo) -> Option<u8> {
		let mut answer = String::from("");

		loop {
			let mut s = String::from(query);
			s.push(' ');
			s.push_str(&answer);

			let mut msgs = VecDeque::new();
			msgs.push_front(s);
			self.write_screen(&mut msgs, sbi);

			match self.wait_for_key_input() {
				Some('\n') => { break; },
				Some(BACKSPACE_CH) => { answer.pop(); },
				Some(ch) => { 
					if ch >= '0' && ch <= '9' {
						answer.push(ch);
					}
				},
				None => { return None; },
			}
		}

		if answer.len() == 0 {
			Some(0)
		} else {
			Some(answer.parse::<u8>().unwrap())
		}
	}

	pub fn query_user(&mut self, question: &str, max: u8, sbi: &SidebarInfo) -> Option<String> {
		let mut answer = String::from("");

		loop {
			let mut s = String::from(question);
			s.push(' ');
			s.push_str(&answer);

			let mut msgs = VecDeque::new();
			msgs.push_front(s);
			self.write_screen(&mut msgs, sbi);

			match self.wait_for_key_input() {
				Some('\n') => { break; },
				Some(BACKSPACE_CH) => { answer.pop(); },
				Some(ch) => { 
					if answer.len() < max as usize { 
						answer.push(ch); 
					}
				},
				None => { return None; },
			}
		}

		Some(answer)
	}

	pub fn get_command(&mut self, state: &GameState) -> Cmd {
		loop {
			for event in self.event_pump.poll_iter() {
				match event {
					Event::Quit {..} => { return Cmd::Quit },
					Event::KeyDown {keycode: Some(Keycode::H), keymod: Mod::LCTRLMOD, .. } |
					Event::KeyDown {keycode: Some(Keycode::H), keymod: Mod::RCTRLMOD, .. } => { 
						return Cmd::MsgHistory; 
					},
					Event::TextInput { text:val, .. } => {
						if val == "Q" {
							return Cmd::Quit;	
						} else if val == "i" {
							return Cmd::ShowInventory
						} else if val == "@" {
							return Cmd::ShowCharacterSheet;	
						} else if val == "w" {
							return Cmd::ToggleEquipment;
						} else if val == " " || val == "." {
							return Cmd::Pass;
						} else if val == "B" {
							return Cmd::ToggleHelm;
						} else if val == "q" {
							return Cmd::Quaff;
						} else if val == "f" {
							return Cmd::FireGun;
						} else if val == "r" {
							return Cmd::Reload;
						} else if val == "M" {
							return Cmd::WorldMap;
						} else if val == "R" {
							return Cmd::Read;
						} else if val == "E" {
							return Cmd::Eat;
						}

						if state.player.on_ship {
							if val == "A" {
								return Cmd::ToggleAnchor;
							} else if val == "h" {
								return Cmd::TurnWheelAnticlockwise;
							} else if val == "j" {
								return Cmd::TurnWheelClockwise;
							}
						} else {
							if val == "k" {
								return Cmd::Move(String::from("N"));
							} else if val == "j" {
								return Cmd::Move(String::from("S"));
							} else if val == "l" {
								return Cmd::Move(String::from("E"));
							} else if val == "h" {
								return Cmd::Move(String::from("W"));
							} else if val == "y" {
								return Cmd::Move(String::from("NW"));
							} else if val == "u" {
								return Cmd::Move(String::from("NE"));
							} else if val == "b" {
								return Cmd::Move(String::from("SW"));
							} else if val == "n" {
								return Cmd::Move(String::from("SE"));
							} else if val == "," {
								return Cmd::PickUp;
							} else if val == "d" {
								return Cmd::DropItem;
							} else if val == "s" {
								return Cmd::Search;
							}
						}
					},
					_ => { continue },
				}
			}
    	}
	}

	pub fn pause_for_more(&mut self) {
		loop {
			for event in self.event_pump.poll_iter() {
				// I need to handle a Quit/Exit event here	
				match event {
					Event::KeyDown {keycode: Some(Keycode::Escape), ..} |
					Event::KeyDown {keycode: Some(Keycode::Space), ..} => {
						// It seemed like the ' ' event was still in the queue.
						// I guess a TextInput event along with the KeyDown event?
						self.event_pump.poll_event();
						return;
					},
					_ => continue,
				}
			}
		}
	}

	fn write_line(&mut self, row: i32, line: &str, small_font: bool) {
		let fw: u32;
		let fh: u32;	
		let f: &Font;

		if small_font {
			f = self.sm_font;
			fw = self.sm_font_width;
			fh = self.sm_font_height;
		} else {
			f = self.font;
			fw = self.font_width;
			fh = self.font_height;
		}

		if line.len() == 0 {
			self.canvas
				.fill_rect(Rect::new(0, row * fh as i32, self.screen_width_px, fh))
				.expect("Error line!");

			return;
		}

		let surface = f.render(line)
			.blended(WHITE)
			.expect("Error rendering message line!");
		let texture_creator = self.canvas.texture_creator();
		let texture = texture_creator.create_texture_from_surface(&surface)
			.expect("Error create texture for messsage line!");
		let rect = Rect::new(10, row * fh as i32, line.len() as u32 * fw, fh);
		self.canvas.copy(&texture, None, Some(rect))
			.expect("Error copying message line texture to canvas!");
	}

	// What I should do here but am not is make sure each line will fit on the
	// screen without being cut off. For the moment, I just gotta make sure any
	// lines don't have too many characterse. Something for a post 7DRL world
	// I guess.
	pub fn write_long_msg(&mut self, lines: &Vec<String>, small_text: bool) {
		self.canvas.clear();
		
		let display_lines = (self.screen_height_px / self.sm_font_height) as usize;
		let line_count = lines.len();
		let mut curr_line = 0;
		let mut curr_row = 0;
		while curr_line < line_count {
			self.write_line(curr_row as i32, &lines[curr_line], small_text);
			curr_line += 1;
			curr_row += 1;

			if curr_row == display_lines - 2 && curr_line < line_count {
				self.write_line(curr_row as i32, "", small_text);
				self.write_line(curr_row as i32 + 1, "-- Press space to continue --", small_text);
				self.canvas.present();
				self.pause_for_more();
				curr_row = 0;
				self.canvas.clear();
			}
		}

		self.write_line(curr_row as i32, "", small_text);
		self.write_line(curr_row as i32 + 1, "-- Press space to continue --", small_text);
		self.canvas.present();
		self.pause_for_more();
	}

	pub fn sq_info_for_tile(&self, tile: &map::Tile) -> (char, sdl2::pixels::Color) {
		let ti = match tile {
			map::Tile::Blank => (' ', BLACK),
			map::Tile::Wall => ('#', GREY),
			map::Tile::Tree => ('\u{03D9}', GREEN),
			map::Tile::Dirt => ('.', BROWN),
			map::Tile::Grass => ('\u{0316}', GREEN),
			map::Tile::Player(color) => ('@', *color),
			map::Tile::Water => ('}', LIGHT_BLUE),
			map::Tile::DeepWater => ('}', BLUE),
			map::Tile::WorldEdge => ('}', BLUE),
			map::Tile::Sand => ('.', BEIGE),
			map::Tile::StoneFloor => ('.', GREY),
			map::Tile::Mountain => ('\u{039B}', GREY),
			map::Tile::SnowPeak => ('\u{039B}', WHITE),
			map::Tile::Lava => ('{', BRIGHT_RED),
			map::Tile::Gate => ('#', LIGHT_BLUE),
			map::Tile::Thing(color, ch) => (*ch, *color),
			map::Tile::Separator => ('|', WHITE),
			map::Tile::ShipPart(ch) => (*ch, BROWN),
			map::Tile::Shipwreck(ch, _) => (*ch, BROWN),
			map::Tile::Mast(ch) => (*ch, BROWN),
			map::Tile::Bullet(ch) => (*ch, WHITE),
			map::Tile::OldFirePit => ('"', GREY),
		};

		ti
	}

	fn write_map_sq(&mut self, r: usize, c: usize, tile_info: (char, sdl2::pixels::Color)) {
		let rect = Rect::new(c as i32 * self.sm_font_width as i32, 
			(r as i32 + 1) * self.sm_font_height as i32, self.sm_font_width, self.sm_font_height);

		self.canvas.set_draw_color(BEIGE);
		self.canvas.fill_rect(rect);
		let (ch, char_colour) = tile_info;
			
		let surface = self.sm_font.render_char(ch)
				.blended(char_colour)
				.expect("Error creating character!");  

		let texture_creator = self.canvas.texture_creator();
		let texture = texture_creator.create_texture_from_surface(&surface)
			.expect("Error creating texture!");

		self.canvas.set_draw_color(BLACK);

		self.canvas.copy(&texture, None, Some(rect))
			.expect("Error copying to canvas!");
	}

	fn write_sq(&mut self, r: usize, c: usize, tile_info: (char, sdl2::pixels::Color)) {
		let (ch, char_colour) = tile_info;

		if !self.surface_cache.contains_key(&tile_info) {
			let s = self.font.render_char(ch)
				.blended(char_colour)
				.expect("Error creating character!");  
			self.surface_cache.insert(tile_info, s);
		}
		let surface = self.surface_cache.get(&tile_info).unwrap();

		let texture_creator = self.canvas.texture_creator();
		let texture = texture_creator.create_texture_from_surface(&surface)
			.expect("Error creating texture!");
		let rect = Rect::new(c as i32 * self.font_width as i32, 
			(r as i32 + 1) * self.font_height as i32, self.font_width, self.font_height);
		self.canvas.copy(&texture, None, Some(rect))
			.expect("Error copying to canvas!");
	}

	fn write_sidebar_line(&mut self, line: &str, start_x: i32, row: u32, colour: sdl2::pixels::Color) {
		let surface = self.font.render(line)
			.blended(colour)
			.expect("Error rendering sidebar!");
		let texture_creator = self.canvas.texture_creator();
		let texture = texture_creator.create_texture_from_surface(&surface)
			.expect("Error creating texture for sdebar!");
		let rect = Rect::new(start_x, (self.font_height * row) as i32, 
			line.len() as u32 * self.font_width, self.font_height);
		self.canvas.copy(&texture, None, Some(rect))
			.expect("Error copying sbi to canvas!");
	}

	fn write_sidebar(&mut self, sbi: &SidebarInfo) {
		let fov_w = (FOV_WIDTH + 1) as i32 * self.font_width as i32; 

		self.write_sidebar_line(&sbi.name, fov_w, 1, WHITE);

		let s = format!("AC: {}", sbi.ac);
		self.write_sidebar_line(&s, fov_w, 2, WHITE);

		let s = format!("Stamina: {}({})", sbi.curr_hp, sbi.max_hp);
		self.write_sidebar_line(&s, fov_w, 3, WHITE);

		let s = format!("Turn: {}", sbi.turn);
		self.write_sidebar_line(&s, fov_w, 21, WHITE);

		if sbi.bearing > -1 {
			let mut s = String::from("Bearing: ");
			match sbi.bearing {
				0 => s.push_str("N"),
				1 => s.push_str("NNE"),
				2 => s.push_str("NE"),
				3 => s.push_str("ENE"),
				4 => s.push_str("E"),
				5 => s.push_str("ESE"),
				6 => s.push_str("SE"),
				7 => s.push_str("SSE"),
				8 => s.push_str("S"),
				9 => s.push_str("SSW"),
				10 => s.push_str("SW"),
				11 => s.push_str("WSW"),
				12 => s.push_str("W"),
				13 => s.push_str("WNW"),
				14 => s.push_str("NW"),
				15 => s.push_str("NNW"),
				_ => s.push_str(""),
			}

			self.write_sidebar_line(&s, fov_w, 5, WHITE);

			let s = "      \\|/".to_string();
			self.write_sidebar_line(&s, fov_w, 7, BROWN);
			
			let s = "      -o-".to_string();
			self.write_sidebar_line(&s, fov_w, 8, BROWN);

			let s = "      /|\\".to_string();
			self.write_sidebar_line(&s, fov_w, 9, BROWN);

			if sbi.wheel == 0 {
				self.write_sq(6, FOV_WIDTH + 8, ('|', GREY));
			} else if sbi.wheel == -1 {
				self.write_sq(6, FOV_WIDTH + 7, ('\\', GREY));
			} else if sbi.wheel == 1 {
				self.write_sq(6, FOV_WIDTH + 9, ('/', GREY));
			} else if sbi.wheel == 2 {
				self.write_sq(7, FOV_WIDTH + 9, ('-', GREY));
			} else if sbi.wheel == -2 {
				self.write_sq(7, FOV_WIDTH + 7, ('-', GREY));
			}
		}
	}

	fn draw_frame(&mut self, msg: &str, sbi: &SidebarInfo) {
		self.canvas.set_draw_color(BLACK);
		self.canvas.clear();

		self.write_line(0, msg, false);
		for row in 0..FOV_HEIGHT {
			for col in 0..FOV_WIDTH {
				let ti = self.sq_info_for_tile(&self.v_matrix[row][col]);
				self.write_sq(row, col, ti);
			}
			self.write_sq(row, FOV_WIDTH, self.sq_info_for_tile(&map::Tile::Separator));
		}

		if sbi.name != "" {
			self.write_sidebar(sbi);
		}

		self.canvas.present();
	}

	pub fn write_screen(&mut self, msgs: &mut VecDeque<String>, sbi: &SidebarInfo) {
		if msgs.len() == 0 {
			self.draw_frame("", sbi);
		} else {
			let mut words = VecDeque::new();
			while msgs.len() > 0 {
				let line = msgs.pop_front().unwrap();
				for w in line.split(" ") {
					let s = String::from(w);
					words.push_back(s);
				}
			}

			let mut s = String::from("");
			while words.len() > 0 {
				let word = words.pop_front().unwrap();

				// If we can't fit the new word in the message put it back
				// on the queue and display what we have so far
				if s.len() + word.len() + 1 >=  SCREEN_WIDTH as usize - 9 {
					words.push_front(word);
					s.push_str("--More--");
					self.draw_frame(&s, sbi);
					self.pause_for_more();
					s = String::from("");	
				} else {
					s.push_str(&word);
					s.push(' ');
				}
			}

			if s.len() > 0 {
				self.draw_frame(&s, sbi);
			}
		}
	}

	// Making the assumption I'll never display a menu with more options than there are 
	// lines on the screen...
	pub fn menu_picker(&mut self, menu: &Vec<String>, answer_count: u8,
				single_choice: bool, small_font: bool) -> Option<HashSet<u8>> {
		let mut answers: HashSet<u8> = HashSet::new();

		loop {
			self.canvas.clear();
			for line in 0..menu.len() {
				if line > 0 && answers.contains(&(line as u8 - 1)) {
					let mut s = String::from("\u{2713} ");
					s.push_str(&menu[line]);
					self.write_line(line as i32, &s, small_font);
				} else {
					self.write_line(line as i32, &menu[line], small_font);
				}
			}
	
			self.write_line(menu.len() as i32 + 1, "", small_font);	
			if !single_choice {
				self.write_line(menu.len() as i32 + 2, "Select one or more options, then hit Return.", small_font);	
			}

			self.canvas.present();

			let a_val = 'a' as u8;
			let answer = self.wait_for_key_input();
			if single_choice {
				match answer {
					None => return None, 	// Esc was pressed, propagate it. 
											// Not sure if thers's a more Rustic way to do this
					Some(v) => {
						if (v as u8) >= a_val && (v as u8) - a_val < answer_count {
							let a = v as u8 - a_val;
							answers.insert(a);
							return Some(answers);
						}	
					}
				}
			} else {
				match answer {
					None => return None, 	// Esc was pressed, propagate it. 
											// Not sure if thers's a more Rustic way to do this
					Some(v) => {
						// * means select everything
						if v == '*' {
							for j in 0..answer_count - 1 {
								answers.insert(j);
							}
							break;
						}
						if (v as u8) >= a_val && (v as u8) - a_val < answer_count {
							let a = v as u8 - a_val;
							
							if answers.contains(&a) {
								answers.remove(&a);
							} else {
								answers.insert(a);
							}
						} else if v == '\n' || v == ' ' {
							break;
						}	
					}
				}
			}
		}

		Some(answers)
	}
}

