use std::collections::{HashMap, HashSet, VecDeque};

use sdl2::pixels::Color;

pub trait TileInfo {
	fn get_tile_info(&self) -> (Color, char);
}

#[derive(Debug, Copy, Clone)]
pub enum ItemType {
	Weapon,
	Clothing,
	Drink,
}

#[derive(Debug)]
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

	pub fn remove_count(&mut self, slot: char, count: u8) -> Vec<Item> {
		let mut items = Vec::new();
		let entry = self.inv.remove_entry(&slot).unwrap();
		let mut v = entry.1;

		let max = if count < v.1 {
			v.1 -= count;
			let mut replacement = ( Item { name: v.0.name.clone(), ..v.0 }, v.1 );
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

	pub fn count_in_slot(&mut self, slot: char) -> u8 {
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
				s.push_str(&val.0.name);
			} else {
				s.push_str(&val.0.name);
				s.push_str(" x");
				s.push_str(&val.1.to_string());
			}
			menu.push(s);
		}

		menu
	}
}

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

	pub fn count_at(&self, r: usize, c: usize) -> u8 {
		let res = if !self.table.contains_key(&(r, c)) {
			0
		} else {
			self.table[&(r, c)].len()
		};

		res as u8
	}

	pub fn peek_top(&self, r: usize, c: usize) -> &Item {
		let stack = self.table.get(&(r, c)).unwrap();
		stack.front().unwrap()
	}

	pub fn get_at(&mut self, r: usize, c: usize) -> Item {
		let mut stack = self.table.get_mut(&(r, c)).unwrap();
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
		let mut stack = self.table.get_mut(&(r, c)).unwrap();
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

#[derive(Debug)]
pub struct Item {
	pub name: String,
	pub item_type: ItemType,
	pub weight: u8,
	pub symbol: char,
	pub color: Color,
	pub stackable: bool,
	pub prev_slot: char
}

impl Item {
	pub fn new(name: &str, item_type: ItemType, w: u8, stackable: bool,
			sym: char, color: Color) -> Item {
		Item { name: String::from(name), 
			item_type, weight: w, symbol: sym, color, stackable, prev_slot: '\0' }
	}
}

impl TileInfo for Item {
	// basically a duplicate of the same method for the Act trait in actor.rs
	// but I don't think having my NPCs list in the main program be a vec of TileInfos
	// insteaf of Act will work for the purposes I want to use it for ;/
	fn get_tile_info(&self) -> (Color, char) {
		(self.color, self.symbol)
	}
}

impl PartialEq for Item {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

