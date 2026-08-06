#![no_std]
#![feature(allocator_api)]
#![feature(btreemap_alloc)]
#![feature(iter_collect_into)]
#![feature(option_into_flat_iter)]
#![feature(exact_div)]
#![feature(slice_ptr_get)]
#![feature(super_let)]

// writeln! generates these which is very annoying
#![allow(unused_must_use)]

// TODO: clean these up
#![allow(unused_imports)]


use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use dyshared::{CAllocator, Page, screen::Screen};
use paging::{MappedPageAllocator, TransToPhys};
use core::fmt::{Debug, Write};

use crate::elf::load_mod;

extern crate alloc;
extern crate dyshared;

mod elf;
mod data_repr;
const FILE_A: &'static [u8] = include_bytes!("../../../user/a");
const FILE_B: &'static [u8] = include_bytes!("../../../user/b");
pub fn test_load_a<LA: CAllocator, PM: MappedPageAllocator>(la: LA, da: &mut PM) -> *mut u8 {
    let mut x: BTreeMap<&str, &[u8], _> = BTreeMap::new_in(la.clone());
    x.insert("a", FILE_A);
    let x = load_mod("a", &x, elf::AllocationStrategy::Exact, la, da);

    return x.fhdr.e_entry as *mut u8
}

pub fn load_higher_half<LA: CAllocator, PM: MappedPageAllocator + Debug>(data: &BTreeMap<&str, &[u8]>, la: LA, da: &mut PM) -> (*mut u8, Box::<[Page], PM::A>) {
    writeln!(Screen, "data is: {:?}", data.keys());
    let out = load_mod("libtest_mod.so", data, elf::AllocationStrategy::ArbitraryKernel, la, da);
    writeln!(Screen, "data is: {:?}", out);
    let (symbol, relocation) = &out.symbols["higher_half_entry"];
    (relocation.relocate_ptr(symbol.st_value as u32), out.allocation)
}