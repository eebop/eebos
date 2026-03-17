#![no_std]
#![feature(allocator_api)]
#![feature(btreemap_alloc)]
#![feature(iter_collect_into)]
#![feature(option_into_flat_iter)]

use alloc::collections::btree_map::BTreeMap;
use dyshared::{CAllocator, screen::Screen};
use paging::{MappedPageAllocator, TransToPhys};
use core::fmt::Write;

use crate::elf::load_mod;

extern crate alloc;
extern crate dyshared;

mod elf;
const FILE_A: &'static [u8] = include_bytes!("../../../user/a");
const FILE_B: &'static [u8] = include_bytes!("../../../user/b");
pub fn test_load_a<LA: CAllocator, PM: MappedPageAllocator>(la: LA, da: &mut PM) -> *mut u8 {
    let mut x: BTreeMap<&str, &[u8], _> = BTreeMap::new_in(la.clone());
    x.insert("a", FILE_A);
    let x = load_mod("a", &x, la, da);

    return x.fhdr.e_entry as *mut u8
}

pub fn load_higher_half<LA: CAllocator, PM: MappedPageAllocator>(data: &BTreeMap<&str, &[u8]>, la: LA, da: &mut PM) {
    let out = load_mod("test_mod", data, la, da);
}