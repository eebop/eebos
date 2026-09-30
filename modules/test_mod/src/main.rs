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
use core::pin::Pin;
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
use alloc::sync::Arc;
use dyshared::{HEAP_PTR, Page, PageAligned, SimpleAllocator, bochsdbg};
use paging::{PageMap, PageType};
use dyshared::screen::Screen;
// use dyshared::cpuinfo::get_cr0;
// use test_dep::test;

#[repr(align(0x1000))]
struct TestData(u32);


#[unsafe(no_mangle)]
pub extern "C" fn _start(ptr: NonNull<u8>) -> ! { 
    unsafe {
        HEAP_PTR = ptr;
    }
    // unsafe extern "C" {
    //     safe static stack_top: u8;
    // }

    // core::hint::black_box(&stack_top);

    // let a: SimpleAllocator = SimpleAllocator;

    // #[derive(Debug)]
    // struct T(u32);

    // impl Default for T {
    //     fn default() -> Self {
    //         T(42)
    //     }
    // }

    // let x = core::hint::black_box(unsafe { 
    //     Pin::new_unchecked(Box::new_in(core::hint::black_box(T::default()), a.clone()))
    // });

    // writeln!(Screen, "here, val is {:?}", x);
    // let a = core::hint::black_box(SimpleAllocator);
    // let raw = Box::new_in([0; 1024], a.clone());

    // core::hint::black_box(raw);

    main();
    // let x = Arc::new(0);
    // core::hint::black_box(x.clone());
    // loop {}
}


// #[unsafe(no_mangle)]
// pub fn higher_half_entry() -> ! {
//     writeln!(Screen, "Now here in higher_half_entry!");
//     loop {}
// }



async fn test() {
    writeln!(Screen, "in async");
    // test().await
}

fn main() -> ! {
    let mut pt_a = paging::page32::PageMap32::new(SimpleAllocator);

    pt_a.insert_many(core::ptr::null_mut(), core::ptr::null_mut(), 8192, paging::PageType::Write, paging::Permission::Supervisor).unwrap();
    let mut pt_b = pt_a.clone();

    // let mut x = Box::new_in(TestData(0), SimpleAllocator);
    // let mut y = Box::new_in(TestData(1), SimpleAllocator);
    


    // pagetable.remove_phys(&raw mut *x as *mut Page).unwrap();
    // pagetable.insert_phys(&raw mut *x as *mut Page, &raw mut *y as *mut Page, paging::PageType::Write, paging::Permission::Supervisor).unwrap();



    let map = paging::IdentityMap;
    let mut da_a = pt_a.allocate_and_page(SimpleAllocator, paging::Permission::Supervisor, &map);
    let mut da_b = pt_b.allocate_and_page(SimpleAllocator, paging::Permission::Supervisor, &map);

    let val_a = process::test_load_a(SimpleAllocator, &mut da_a);
    let val_b = process::test_load_b(SimpleAllocator, &mut da_b);

    // let (ptr, data) = process::load_higher_half(&data, SimpleAllocator, &mut da);

    let mut token = unsafe { paging::PageToken::new() };
    unsafe {
        token = pt_a.build(token);
    };


    // interrupts::test(SimpleAllocator);
    // loop {}
    let ptr_a: extern "C" fn() -> ! = unsafe { core::mem::transmute(val_a) };
    // let ptr_b: extern "C" fn() -> ! = unsafe { core::mem::transmute(val_b) };
    
    // ptr();
    let mut stack_a = Page::uninit_many(4, SimpleAllocator);
    let mut stack_b = Page::uninit_many(4, SimpleAllocator);
    let stack_a_top = (&mut *stack_a).as_contiguous().as_ptr_range().end as *mut u32;
    let stack_b_top = (&mut *stack_b).as_contiguous().as_ptr_range().end as *mut u32;
    // tasks::Task::from_fn(ptr_a, stack_a_top, pt_a);
    // tasks::Task::from_fn(ptr_a, stack_b_top, pt_b);

    let t = interrupts::runtime::Task::from_async(test(), stack_a_top, pt_a);

    let mut rt = interrupts::runtime::Schedule::new(token, Arc::new(t));
    interrupts::runtime::RT = rt;
}
