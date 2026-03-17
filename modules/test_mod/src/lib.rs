#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(ptr_mask)]

use core::alloc::{Allocator, GlobalAlloc, Layout};
use core::num::NonZero;
use core::panic::PanicInfo;

use core::arch::asm;
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
use dyshared::Page;
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
fn main(ptr: *mut u8, _: u8/*, data: *const BTreeMap<&str, &[u8]>*/) {
    writeln!(Screen::new(), "ptr is: {:?} (in main)", ptr);
    unsafe {DATAPTR = NonNull::new_unchecked(ptr) }
    let mut pagetable = paging::page32::PageMap32::new(SimpleAllocator);

    writeln!(Screen::new(), "calling test_dep...");

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


    unsafe {
        let mut val = paging::PageToken::new();
        pagetable.build(val);

    };
    writeln!(Screen::new(), "val ({:?}) is: {}", &raw const x.0, x.0);

    let val = unsafe { core::mem::transmute::<_, extern "C" fn()>(val) };
    val();

    loop {}
}
