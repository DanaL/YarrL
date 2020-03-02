extern crate sdl2;

use std::collections::{HashSet, VecDeque};

use crate::map;
use super::{Cmd, Map, FOV_WIDTH, FOV_HEIGHT};

use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Mod;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::ttf::Font;
use sdl2::pixels::Color;

pub static BLACK: Color = Color::RGBA(0, 0, 0, 255);
pub static WHITE: Color = Color::RGBA(255, 255, 255, 255);
pub static GREY: Color = Color::RGBA(136, 136, 136, 255);
pub static GREEN: Color = Color::RGBA(46, 139, 87, 255);
pub static BROWN: Color = Color::RGBA(153, 0, 0, 255);
pub static BLUE: Color = Color::RGBA(0, 0, 221, 255);
pub static LIGHT_BLUE: Color = Color::RGBA(55, 198, 255, 255);
pub static BEIGE: Color = Color::RGBA(255, 178, 127, 255);

const SCREEN_WIDTH: u32 = 49;
const SCREEN_HEIGHT: u32 = 22;
const BACKSPACE_CH: char = '\u{0008}';

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
}

impl<'a, 'b> GameUI<'a, 'b> {
	pub fn init(font: &'b Font, sm_font: &'b Font) -> Result<GameUI<'a, 'b>, String> {
		let (font_width, font_height) = font.size_of_char(' ').unwrap();
		let screen_width_px = SCREEN_WIDTH * font_width;
		let screen_height_px = SCREEN_HEIGHT * font_height;

		let (sm_font_width, sm_font_height) = sm_font.size_of_char(' ').unwrap();

		let sdl_context = sdl2::init()?;
		let video_subsystem = sdl_context.video()?;
		let window = video_subsystem.window("RL Demo", screen_width_px, screen_height_px)
			.position_centered()
			.opengl()
			.build()
			.map_err(|e| e.to_string())?;

		let v_matrix = vec![vec![map::Tile::Blank; FOV_WIDTH]; FOV_HEIGHT];
		let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
		let mut gui = GameUI { 
			screen_width_px, screen_height_px, 
			font, font_width, font_height, 
			canvas,
			event_pump: sdl_context.event_pump().unwrap(),
			sm_font, sm_font_width, sm_font_height,
			v_matrix,
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

	pub fn query_single_response(&mut self, question: &str) -> Option<char> {
		let mut m = VecDeque::new();
		m.push_front(question.to_string());
		self.write_screen(&mut m);

		self.wait_for_key_input()
	}

	pub fn query_natural_num(&mut self, query: &str) -> Option<u8> {
		let mut answer = String::from("");

		loop {
			let mut s = String::from(query);
			s.push(' ');
			s.push_str(&answer);

			let mut msgs = VecDeque::new();
			msgs.push_front(s);
			self.write_screen(&mut msgs);

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

	pub fn query_user(&mut self, question: &str) -> String {
		let mut answer = String::from("");

		loop {
			let mut s = String::from(question);
			s.push(' ');
			s.push_str(&answer);

			let mut msgs = VecDeque::new();
			msgs.push_front(s);
			self.write_screen(&mut msgs);

			let ch = self.wait_for_key_input().unwrap();
			match ch {
				'\n' => { break; },
				BACKSPACE_CH => { answer.pop(); },
				_ => { answer.push(ch); },
			}
		}

		answer
	}

	pub fn get_command(&mut self) -> Cmd {
		loop {
			for event in self.event_pump.poll_iter() {
				match event {
					Event::KeyDown {keycode: Some(Keycode::Escape), ..} 
						| Event::Quit {..} => { return Cmd::Exit },
					Event::KeyDown {keycode: Some(Keycode::H), keymod: Mod::LCTRLMOD, .. } |
					Event::KeyDown {keycode: Some(Keycode::H), keymod: Mod::RCTRLMOD, .. } => { 
						return Cmd::MsgHistory; 
					},
					Event::TextInput { text:val, .. } => {
						if val == "Q" {
							return Cmd::Exit;	
						} else if val == "k" {
							return Cmd::MoveN;
						} else if val == "j" {
							return Cmd::MoveS;
						} else if val == "l" {
							return Cmd::MoveE;
						} else if val == "h" {
							return Cmd::MoveW;
						} else if val == "y" {
							return Cmd::MoveNW;
						} else if val == "u" {
							return Cmd::MoveNE;
						} else if val == "b" {
							return Cmd::MoveSW;
						} else if val == "n" {
							return Cmd::MoveSE;
						} else if val == "," {
							return Cmd::PickUp;
						} else if val == "i" {
							return Cmd::ShowInventory
						} else if val == "d" {
							return Cmd::DropItem;
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
		let rect = Rect::new(0, row * fh as i32, line.len() as u32 * fw, fh);
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

	fn write_sq(&mut self, r: usize, c: usize, tile: map::Tile) {
		let (ch, char_colour) = match tile {
			map::Tile::Blank => (' ', BLACK),
			map::Tile::Wall => ('#', GREY),
			map::Tile::Tree => ('\u{03D9}', GREEN),
			map::Tile::Dirt => ('.', BROWN),
			map::Tile::Grass => ('\u{0316}', GREEN),
			map::Tile::Player => ('@', WHITE),
			map::Tile::Water => ('}', LIGHT_BLUE),
			map::Tile::DeepWater => ('}', BLUE),
			map::Tile::Sand => ('.', BEIGE),
			map::Tile::StoneFloor => ('.', GREY),
			map::Tile::Mountain => ('^', GREY),
			map::Tile::SnowPeak => ('^', WHITE),
			map::Tile::Gate => ('#', LIGHT_BLUE),
			map::Tile::Thing(color, ch) => (ch, color),
		};

		let surface = self.font.render_char(ch)
			.blended(char_colour)
			.expect("Error creating character!");  
		let texture_creator = self.canvas.texture_creator();
		let texture = texture_creator.create_texture_from_surface(&surface)
			.expect("Error creating texture!");
		let rect = Rect::new(c as i32 * self.font_width as i32, 
			(r as i32 + 1) * self.font_height as i32, self.font_width, self.font_height);
		self.canvas.copy(&texture, None, Some(rect))
			.expect("Error copying to canvas!");
	}

	fn draw_frame(&mut self, msg: &str) {
		self.canvas.set_draw_color(BLACK);
		self.canvas.clear();

		self.write_line(0, msg, false);
		for row in 0..FOV_HEIGHT {
			for col in 0..FOV_WIDTH {
				self.write_sq(row, col, self.v_matrix[row][col]);
			}
		}

		self.canvas.present();
	}

	pub fn write_screen(&mut self, msgs: &mut VecDeque<String>) {
		if msgs.len() == 0 {
			self.draw_frame("");
		} else {
			let mut s = String::from("");
			let mut draw = false;
			loop {
				if msgs.len() == 0 {
					self.draw_frame(&s);
					break;
				} 

				let msg = msgs.get(0).unwrap();
				if s.len() + msg.len() < SCREEN_WIDTH as usize - 9 {
					s.push_str(msg);
					s.push_str(" ");
					msgs.pop_front();
				} else {
					s.push_str("--More--");
					self.draw_frame(&s);
					self.pause_for_more();
					s = String::from("");
				}
			}
		}
	}

	// Making the assumption I'll never display a menu with more options than there are 
	// lines on the screen...
	pub fn menu_picker(&mut self, menu: &Vec<String>, answer_count: u8) -> Option<HashSet<u8>> {
		let mut answers: HashSet<u8> = HashSet::new();

		loop {
			self.canvas.clear();
			for line in 0..menu.len() {
				if line > 0 && answers.contains(&(line as u8 - 1)) {
					let mut s = String::from("\u{2713} ");
					s.push_str(&menu[line]);
					self.write_line(line as i32, &s, false);
				} else {
					self.write_line(line as i32, &menu[line], false);
				}
			}
	
			self.write_line(menu.len() as i32 + 1, "", false);	
			self.write_line(menu.len() as i32 + 2, "Select one or more options, then hit Return.", false);	
			self.canvas.present();

			let a_val = 'a' as u8;
			let answer = self.wait_for_key_input();
			match answer {
				None => return None, 	// Esc was pressed, propagate it. 
										// Not sure if thers's a more Rustic way to do this
				Some(v) => {
					// * is select everything
					if v == '*' {
						for j in 0..answer_count - 1 {
							answers.insert(j);
						}
						break;
					}
					if (v as u8) >= a_val || (v as u8) < answer_count {
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

		Some(answers)
	}
}

