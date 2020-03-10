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

use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Serialize, Deserialize};

use crate::display;

pub trait TileInfo {
	fn get_tile_info(&self) -> ((u8, u8, u8), char);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
	next_slot: char,
	inv: HashMap<char, (Item, u8)>,
}

impl Inventory {
	pub fn new() -> Inventory {
		Inventory { next_slot: 'a', inv: HashMap::new() }
	}

	fn set_next_slot(&mut self) {
		let mut slot = self.next_slot;
		
		loop {
			slot = (slot as u8 + 1) as char;
			if slot > 'z' {
				slot = 'a';
			}

			if !self.inv.contains_key(&slot) {
				self.next_slot = slot;
				break;
			}

			if slot == self.next_slot {
				// No free spaces left in the invetory!
				self.next_slot = '\0';
				break;
			}
		}
	}

	// It's such a pain handling mutable refs in nested data
	// structures in Rust that I'm just going to do this. I guess
	// it's a message passing pattern, only terrible :/
	fn toggle_loaded_status(&mut self) {
		let mut gun_slot = '\0';
		for slot in self.inv.keys() {
			let w = self.inv.get(&slot).unwrap();
			if w.0.equiped && w.0.item_type == ItemType::Firearm {
				gun_slot = *slot;
			}
		}

		if gun_slot != '\0' {
			let gun = self.inv.get_mut(&gun_slot).unwrap();
			gun.0.loaded = !gun.0.loaded;
		}	
	}

	pub fn firearm_fired(&mut self) {
		self.toggle_loaded_status();
	}

	pub fn reload_firearm(&mut self) {
		self.toggle_loaded_status();
	}

	pub fn equiped_magic_eye_patch(&self) -> bool {
		for slot in self.inv.keys() {
			let w = self.inv.get(&slot).unwrap();
			if w.0.equiped && w.0.name == "magic eye patch" {
				return true;
			}
		}

		false
	}

	pub fn get_equiped_firearm(&self) -> Option<Item> {
		for slot in self.inv.keys() {
			let w = self.inv.get(&slot).unwrap();
			if w.0.equiped && w.0.item_type == ItemType::Firearm {
				return Some(w.0.clone());
			}
		}

		None
	}
	
	pub fn get_equiped_weapon(&self) -> Option<Item> {
		for slot in self.inv.keys() {
			let w = self.inv.get(&slot).unwrap();
			if w.0.equiped && w.0.item_type == ItemType::Weapon {
				return Some(w.0.clone());
			}
		}

		None
	}

	fn type_already_equiped(&self, i_type: ItemType) -> bool {
		for slot in self.inv.keys() {
			let v = self.inv.get(&slot).unwrap();
			if v.0.item_type == i_type && v.0.equiped {
				return true;
			}
		}

		false
	}

	pub fn total_armour_value(&self) -> i8 {
		let mut sum = 0;
		for slot in self.inv.keys() {
			let v = self.inv.get(&slot).unwrap();
			if v.0.equiped {
				sum += v.0.armour_value;
			}
		}

		sum
	}
 
	pub fn toggle_slot(&mut self, slot: char) -> String {
		if !self.inv.contains_key(&slot) {
			return String::from("You do not have that item!");
		}

		let val = self.inv.get(&slot).unwrap();
		let item = &val.0;

		if !item.equipable() {
			return String::from("You cannot equip that!");
		}

		if !item.equiped && self.type_already_equiped(item.item_type) {
			return match item.item_type {
				ItemType::Weapon => String::from("You are already holding a weapon."),
				ItemType::Firearm => String::from("You are already holding a gun."),
				ItemType::Hat => String::from("You are already wearing a hat."),
				ItemType::Coat => String::from("You are already wearing a coat."),
				ItemType::EyePatch => String::from("You are already wearing an eye patch."),
				_ => panic!("We shouldn't hit this option"),
			};
		}

		// Okay, at this point we are either toggling or untoggling the item so
		// I can take a fucking mutable borrow without the borrow checking flipping out
		let val = self.inv.get_mut(&slot).unwrap();
		let mut item = &mut val.0;

		item.equiped = !item.equiped;

		let mut s = String::from("You ");
		if item.equiped {
			s.push_str("equip the ");
		} else {
			s.push_str("unequip the ");
		}
		s.push_str(&item.name);
		s.push('.');

		s
	}

	pub fn find_ammo(&mut self) -> bool {
		// sigh...
		let slots = self.inv.keys()
					.map(|s| s.clone())
					.collect::<Vec<char>>();

		for s in slots {
			if self.item_type_in_slot(s).unwrap() == ItemType::Bullet {
				self.remove_count(s, 1);
				return true;
			}
		}

		false
	}

	pub fn remove_count(&mut self, slot: char, count: u8) -> Vec<Item> {
		let mut items = Vec::new();
		let entry = self.inv.remove_entry(&slot).unwrap();
		let mut v = entry.1;

		let max = if count < v.1 {
			v.1 -= count;
			let replacement = ( Item { name: v.0.name.clone(), ..v.0 }, v.1 );
			self.inv.insert(slot, replacement);
			count	
		} else {
			if self.next_slot == '\0' {
				self.next_slot = slot;
			}
			v.1
		};

		for _ in 0..max {
			let mut i = Item { name:v.0.name.clone(), ..v.0 }; 
			i.prev_slot = slot;
			items.push(i);
		}

		items
	}

	// Again, I'm leaving it up to the caller to ensure the slot exists.
	// Bad for a library but maybe okay for my internal game code
	pub fn remove(&mut self, slot: char) -> Item {
		let mut v = self.inv.remove(&slot).unwrap();
		if self.next_slot == '\0' {
			self.next_slot = slot;
		}
		v.0.prev_slot = slot;

		v.0
	}

	pub fn item_type_in_slot(&self, slot: char) -> Option<ItemType> {
		if !self.inv.contains_key(&slot) {
			None
		} else {
			let v = self.inv.get(&slot).unwrap();
			Some(v.0.item_type)
		}
	}

	pub fn peek_at(&self, slot: char) -> Option<&Item> {
		if !self.inv.contains_key(&slot) {
			None
		} else {
			let v = self.inv.get(&slot).unwrap();
			Some(&v.0)
		}
	}

	pub fn count_in_slot(&self, slot: char) -> u8 {
		if !self.inv.contains_key(&slot) {
			0
		} else {
			let v = self.inv.get(&slot).unwrap();
			v.1
		}
	}

	pub fn add(&mut self, item: Item) {
		if item.stackable {
			// since the item is stackable, let's see if there's a stack we can add it to
			// Super cool normal programming language way to loop over the keys of a hashtable :?
			let slots = self.inv.keys()
								.map(|v| v.clone())
								.collect::<Vec<char>>();
			for slot in slots {
				let mut val = self.inv.get_mut(&slot).unwrap();
				if val.0 == item {
					val.1 += 1;
					return;
				}
			}
		} 

		// If the last slot the item occupied is still available, use that
		// instead of the next available slot.
		if item.prev_slot != '\0' && !self.inv.contains_key(&item.prev_slot) {
			self.inv.insert(item.prev_slot, (item, 1));
		} else {
			self.inv.insert(self.next_slot, (item, 1));
			self.set_next_slot();
		}
	}

	pub fn get_menu(&self) -> Vec<String> {
		let mut menu = Vec::new();

		let mut slots = self.inv
			.keys()
			.map(|v| v.clone())
			.collect::<Vec<char>>();
		slots.sort();

		for slot in slots {
			let mut s = String::from("");
			s.push(slot);
			s.push_str(") ");
			let val = self.inv.get(&slot).unwrap();
			if val.1 == 1 {
				s.push_str("a ");
				s.push_str(&val.0.get_full_name());
			} else {
				s.push_str(&val.0.get_full_name());
				s.push_str(" x");
				s.push_str(&val.1.to_string());
			}
			menu.push(s);
		}

		menu
	}
}

#[derive(Serialize, Deserialize)]
pub struct ItemsTable {
	table: HashMap<(usize, usize), VecDeque<Item>>,
}

impl ItemsTable {
	pub fn new() -> ItemsTable {
		ItemsTable { table: HashMap::new() }
	}

	pub fn add(&mut self, r: usize, c: usize, item: Item) {
		if !self.table.contains_key(&(r, c)) {
			self.table.insert((r, c,), VecDeque::new());
		}

		let stack = self.table.get_mut(&(r, c)).unwrap();
		stack.push_front(item);
	}

	pub fn reveal_hidden(&mut self, loc: &(usize, usize)) {
		if !self.table.contains_key(loc) {
			return;
		}

		let pile = self.table.get_mut(loc).unwrap();
		for item in pile {
			item.hidden = false;
		}
	}

	pub fn macguffin_here(&self, loc: &(usize, usize)) -> bool {
		if !self.table.contains_key(loc) {
			return false;
		}

		let pile = &self.table[&(loc.0, loc.1)];
		for item in pile {
			if item.item_type == ItemType::MacGuffin { 
				return true
			}
		}
		
		false
	}

	pub fn any_hidden(&self, loc: &(usize, usize)) -> bool {
		if !self.table.contains_key(loc) {
			return false;
		}

		let pile = &self.table[&(loc.0, loc.1)];
		for item in pile {
			if item.hidden { 
				return true
			}
		}
		
		false
	}
 
	fn count_visible(&self, loc: (usize, usize)) -> usize {
		let mut count = 0;
		let pile = &self.table[&(loc.0, loc.1)];
		for item in pile {
			if !item.hidden { 
				count += 1; 
			}
		}
		
		count
	}

	pub fn count_at(&self, r: usize, c: usize) -> u8 {
		let res = if !self.table.contains_key(&(r, c)) {
			0
		} else {
			self.count_visible((r, c))
		};

		res as u8
	}

	pub fn peek_top(&self, r: usize, c: usize) -> &Item {
		let stack = self.table.get(&(r, c)).unwrap();
		stack.front().unwrap()
	}

	pub fn get_at(&mut self, r: usize, c: usize) -> Item {
		let stack = self.table.get_mut(&(r, c)).unwrap();
		stack.pop_front().unwrap()
	}

	// Putting the burden of ensuring slots sent actually exist 
	pub fn get_many_at(&mut self, r: usize, c: usize, slots: &HashSet<u8>) -> Vec<Item> {
		let mut indices = slots.iter()
								.map(|v| *v as usize)
								.collect::<Vec<usize>>();
		indices.sort();
		indices.reverse();

		let mut items = Vec::new();
		let stack = self.table.get_mut(&(r, c)).unwrap();
		for i in indices {
			let item = stack.remove(i).unwrap();
			items.push(item);
		}

		items
	}

	pub fn get_menu(&self, r: usize, c: usize) -> Vec<String> {
		let mut menu = Vec::new();
		let items = self.table.get(&(r, c)).unwrap();
		
		for j in 0..items.len() {
			let mut s = String::from("");
			s.push(('a' as u8 + j as u8) as char);
			s.push_str(") ");
			s.push_str(&items[j].name);
	
			menu.push(s);
		}

		menu
	}
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemType {
	Weapon,
	Coat,
	Hat,
	Drink,
	Firearm,
	Bullet,
	Coin,
	TreasureMap,
	Food,
	EyePatch,
	Note,
	MacGuffin,
}

// Cleaning up this struct and making it less of a dog's 
// breakfast is big on my post-7DRL list of things to do
// Not quite sure yet how to use Traits to achieve something 
// analogous to polymorphism so I can have a list of various 
// items of different categories. Like, doubloons should not
// have an armour value, a bottle of rum doesn't need range
// or loaded attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
	pub name: String,
	pub item_type: ItemType,
	pub weight: u8,
	pub symbol: char,
	pub color: (u8, u8, u8),
	pub stackable: bool,
	pub prev_slot: char,
	pub dmg: u8,
	pub dmg_dice: u8,
	pub bonus: u8,
	pub range: u8,
	pub armour_value: i8,
	pub equiped: bool,
	pub loaded: bool,
	pub hidden: bool,
	pub nw_corner: (usize, usize),
	pub x_coord: (usize, usize),
}

impl Item {
	fn new(name: &str, item_type: ItemType, w: u8, stackable: bool, sym: char, color: (u8, u8, u8)) -> Item {
		Item { name: String::from(name), 
			item_type, weight: w, symbol: sym, color, stackable, prev_slot: '\0',
				dmg: 1, dmg_dice: 1, bonus: 0, range: 0, armour_value: 0, 
				equiped: false, loaded: false, hidden: false, nw_corner: (0, 0),
				x_coord: (0, 0) }
	}

	pub fn get_indefinite_article(&self) -> String {
		if self.item_type == ItemType::MacGuffin {
			return String::from("");
		} else {
			let first = self.name.chars().next().unwrap();
			if first == 'a' || first == 'e' || first == 'i' ||
				first == 'o' || first == 'u' || first == 'y' {
				return String::from("an");
			} else {
				return String::from("a");
			}
		}
	}

	pub fn get_definite_article(&self) -> String {
		if self.item_type == ItemType::MacGuffin {
			return String::from("");
		} else {
			return String::from("the");
		}
	}

	pub fn equipable(&self) -> bool {
		match self.item_type {
			ItemType::Weapon | ItemType::Coat | ItemType::Hat 
				| ItemType::Firearm | ItemType::EyePatch => true,
			_ => false, 
		}
	}

	pub fn get_map(nw_corner: (usize, usize), x_coord: (usize, usize)) -> Item {
		let mut map = Item::new("treasure map", ItemType::TreasureMap, 0, false, '?', display::WHITE);
		map.nw_corner =	nw_corner; 
		map.x_coord = x_coord;

		map
	}

	pub fn get_macguffin(pirate_lord: &str) -> Item {
		let s = format!("{}'s chest", pirate_lord);
		let mut mg = Item::new(&s, ItemType::MacGuffin, 0, false, '=', 
					display::GOLD);
		mg.hidden = true;

		mg
	}
	
	pub fn get_note(note_num: u8) -> Item {
		let mut note = Item::new("scrap of paper", ItemType::Note, 0, false, '?', display::WHITE);
		note.bonus = note_num;

		note
	}

	pub fn get_note_text(ship_name: &str) -> String {
		let mut s = String::from("");
		let r = rand::thread_rng().gen_range(0, 4);
		if r == 0 {
			s.push_str("A ship's manifest from the ");
		} else if r == 1 {
			s.push_str("A love letter addressed to the bosun of the ");
		} else if r == 2 {
			s.push_str("'Wanted for piracy, the crew of the ");
		} else {
			s.push_str("An invoice for 10 barrels of beer for the ");
		}
		s.push_str(ship_name);
		s.push('.');
		if r == 2 {
			s.push_str("'");
		}

		s
	}

	pub fn get_item(name: &str) -> Option<Item> {
		match name {
			"draught of rum" => { 
				let mut r = (Item::new(name, ItemType::Drink, 1, true, '!', display::BROWN));
				r.bonus = 15;
				Some(r)
			},
			"rusty cutlass" => {
				let mut i = Item::new(name, ItemType::Weapon, 3, false, '|', display::WHITE);
				i.dmg = 5;
				Some(i)
			},
			"battered tricorn" => {
				let mut i = Item::new(name, ItemType::Hat, 1, false, '[', display::BROWN);
				i.armour_value = 1;
				Some(i)
			},
			"leather jerkin" => {
				let mut i = Item::new(name, ItemType::Coat, 2, false, '[', display::BROWN);
				i.armour_value = 1;
				Some(i)
			},
			"overcoat" => {
				let mut i = Item::new(name, ItemType::Coat, 3, false, '[', display::BLUE);
				i.armour_value = 2;
				Some(i)
			},
			"magic eye patch" => {
				let mut i = Item::new(name, ItemType::EyePatch, 0, false, '[', display::BRIGHT_RED);
				i.armour_value = 0;
				Some(i)
			},
			"flintlock pistol" => {
				let mut i = Item::new(name, ItemType::Firearm, 2, false, '-', display::GREY);
				i.loaded = true;
				i.dmg = 6;
				i.dmg_dice = 2;
				i.range = 6;
				Some(i)
			},
			"corroded flintlock" => {
				let mut i = Item::new(name, ItemType::Firearm, 2, false, '-', display::GREY);
				i.loaded = false;
				i.dmg = 5;
				i.dmg_dice = 2;
				i.range = 6;
				Some(i)
			},

			"lead ball" => Some(Item::new(name, ItemType::Bullet, 1, true, '*', display::GREY)),
			"doubloon" => Some(Item::new(name, ItemType::Coin, 1, true, '$', display::GOLD)),
			"coconut" => {
				let mut i = Item::new(name, ItemType::Food, 1, true, '%', display::BEIGE);
				i.bonus = 7;
				Some(i)
			},
			"banana" => {
				let mut i = Item::new(name, ItemType::Food, 1, true, '(', display::YELLOW);
				i.bonus = 5;
				Some(i)
			},
			"salted pork" => {
				let mut i = Item::new(name, ItemType::Food, 1, true, '%', display::BROWN);
				i.bonus = 3;
				Some(i)
			},
			_ => None,

		}
	}

	pub fn get_full_name(&self) -> String {
		let mut s = String::from(&self.name);

		if self.equiped {
			match self.item_type {
				ItemType::Weapon | ItemType::Firearm => s.push_str(" (in hand)"),
				ItemType::Coat | ItemType::Hat | ItemType::EyePatch => s.push_str(" (being worn)"),
				_ => panic!("Should never hit this option..."),
			}
		}

		s
	}
}

impl TileInfo for Item {
	// basically a duplicate of the same method for the Act trait in actor.rs
	// but I don't think having my NPCs list in the main program be a vec of TileInfos
	// insteaf of Act will work for the purposes I want to use it for ;/
	fn get_tile_info(&self) -> ((u8, u8, u8), char) {
		(self.color, self.symbol)
	}
}

impl PartialEq for Item {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

