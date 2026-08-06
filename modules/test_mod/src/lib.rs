#![no_std]
#![no_main]
#![feature(allocator_api)]

// writeln! generates these which is very annoying
#![allow(unused_must_use)]

// TODO: clean these up
#![allow(unused_imports)]

use core::alloc::{Allocator, GlobalAlloc, Layout};
use core::num::NonZero;
use core::panic::PanicInfo;

use core::arch::{asm, naked_asm};
use core::ptr::NonNull;
use core::*;
use core::fmt::Write;

extern crate alloc;
extern crate test_dep;
extern crate dyshared;
extern crate paging;
extern crate process;

use alloc::alloc::AllocError;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use dyshared::{Page, bochsdbg};
use paging::{PageMap, PageType};
use dyshared::screen::Screen;
// use dyshared::cpuinfo::get_cr0;
// use test_dep::test;

static mut DATAPTR: NonNull<u8> = core::ptr::NonNull::dangling();

#[derive(Clone, Copy, Debug)]
struct SimpleAllocator;

unsafe impl Allocator for SimpleAllocator {
        
    fn allocate(&self, layout: Layout) -> Result<ptr::NonNull<[u8]>, AllocError> {
        let mask = layout.align() - 1;
        let out = unsafe {
            DATAPTR = DATAPTR.add(mask).map_addr(|addr| NonZero::new(addr.get() & !(mask)).unwrap());
            let out = DATAPTR;

            DATAPTR = DATAPTR.add(layout.size());
            out
        };
        Ok(NonNull::slice_from_raw_parts(out, layout.size()))
    }
    
    unsafe fn deallocate(&self, ptr: ptr::NonNull<u8>, layout: Layout) {
    }
}

#[repr(align(0x1000))]
struct TestData(u32);

#[unsafe(no_mangle)]
pub extern "C" fn start(ptr: *mut u8, data: *const BTreeMap<&str, &[u8]>) -> ! { 
    unsafe {DATAPTR = NonNull::new(ptr).unwrap() }
    let map = unsafe { data.read() };
    main(map);
}


#[unsafe(no_mangle)]
pub fn higher_half_entry() -> ! {
    writeln!(Screen, "Now here in higher_half_entry!");
    loop {}
}

fn main(data: BTreeMap<&str, &[u8]>) -> ! {
    // writeln!(Screen, "ptr is: {:?} (in main)", ptr);

    let mut pagetable = paging::page32::PageMap32::new(SimpleAllocator);

    writeln!(Screen, "calling test_dep...");

    test_dep::test();
    // writeln!(Screen::new(), "cr0 is {}", get_cr0());

    pagetable.insert_many(core::ptr::null_mut(), core::ptr::null_mut(), 8192, paging::PageType::Write, paging::Permission::Supervisor).unwrap();

    let mut x = Box::new_in(TestData(0), SimpleAllocator);
    let mut y = Box::new_in(TestData(1), SimpleAllocator);
    


    pagetable.remove_phys(&raw mut *x as *mut Page).unwrap();
    pagetable.insert_phys(&raw mut *x as *mut Page, &raw mut *y as *mut Page, paging::PageType::Write, paging::Permission::Supervisor).unwrap();


    let map = paging::IdentityMap;
    let mut da = pagetable.allocate_and_page(SimpleAllocator, paging::Permission::Supervisor, &map);

    let val = process::test_load_a(SimpleAllocator, &mut da);
    // let (ptr, data) = process::load_higher_half(&data, SimpleAllocator, &mut da);

    unsafe {
        let token = paging::PageToken::new();
        pagetable.build(token);
    };


    interrupts::test(SimpleAllocator);
    loop {}
    let ptr: fn() -> ! = unsafe { core::mem::transmute(val) };
    ptr();


    // writeln!(Screen, "val ({:?}) is: {}", &raw const x.0, x.0);
    // bochsdbg();
    // let val = unsafe { core::mem::transmute::<_, extern "C" fn()>(val) };
    // val();

    // loop {}
}
