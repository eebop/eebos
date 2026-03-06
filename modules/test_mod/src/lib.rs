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

// mod shared;
// mod test_dep;

// #[link(name="libtest_dep.so", kind="dylib")]
// extern crate test_dep;

extern crate alloc;
extern crate test_dep;
extern crate dyshared;
extern crate paging;

use alloc::alloc::AllocError;
use alloc::boxed::Box;
use dyshared::Page;
use paging::PageMap;
use dyshared::screen::Screen;
// use dyshared::cpuinfo::get_cr0;
// use test_dep::test;

// #[panic_handler]
// fn panic<'a, 'b>(_: &'a PanicInfo<'b>) -> ! {
//     loop {}
// }

// #[unsafe(no_mangle)]
// pub extern "C" fn __libc_start_main() {
//     main();
// }

// struct EmptyAllocator;

// unsafe impl GlobalAlloc for EmptyAllocator {
//     unsafe fn alloc(&self, layout: alloc::Layout) -> *mut u8 {
//         panic!("Alloc not supported here!");
//     }

//     unsafe fn dealloc(&self, ptr: *mut u8, layout: alloc::Layout) {
//         panic!("Alloc not supported here!");
//     }
// }

// #[global_allocator]
// static empty_allocator: EmptyAllocator = EmptyAllocator;

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
pub extern "C" fn main(ptr: *mut u8) -> u32 {
    unsafe {DATAPTR = NonNull::new_unchecked(ptr) }
    let mut pagetable = paging::page32::PageMap32::new(SimpleAllocator);

    writeln!(Screen::new(), "calling test_dep...");
    

    test_dep::test();
    
    // writeln!(Screen::new(), "cr0 is {}", get_cr0());

    pagetable.insert_many(core::ptr::null_mut(), core::ptr::null_mut(), 8192, paging::PageType::Write, paging::Permission::Supervisor).unwrap();
    // writeln!(Screen::new(), "paging is {:?}", x);

    let mut x = Box::new_in(TestData(0), SimpleAllocator);
    let mut y = Box::new_in(TestData(1), SimpleAllocator);
    
    pagetable.insert_phys(&raw mut *x as *mut Page, &raw mut *y as *mut Page, paging::PageType::Write, paging::Permission::Supervisor).unwrap();

    unsafe {
        let mut val = paging::PageToken::new();
        pagetable.build(&mut val);

    };
    writeln!(Screen::new(), "val is: {}", x.0);


    loop {}
}
