#![no_std]
#![feature(pointer_is_aligned_to)]
#![feature(allocator_api)]
#![feature(unsafe_cell_access)]
#![feature(never_type)]

// writeln! generates these which is very annoying
#![allow(unused_must_use)]

// TODO: clean these up
#![allow(unused_imports)]

extern crate alloc;

use core::arch::{asm, naked_asm};

use dyshared::{self, CAllocator, bochsdbg, screen::Screen};
use paging;

use core::fmt::Write;

mod idt;
mod single_threaded;
// mod mutex;
mod interrupts;
pub mod runtime;

// pub enum InterruptType {
//     ClockEvent
// }

// pub fn submit_callback(interrupt: InterruptType) {

// }

// pub fn start_clock() {
//     writeln!(Screen, "val is {:?}", stack_top);
//     pic::enable(0); // PIT
// }


pub fn test<A: CAllocator + 'static>(a: A) {
    idt::init_interrupts(a);

    unsafe extern "C" {
        safe static stack_top: u8;
    }

    writeln!(Screen, "stack_top is {:?}", &raw const stack_top);

    bochsdbg();
    unsafe {
        asm!(
            "xchg bx, bx",
            "int 3",
            "xchg bx, bx",
        )
    }
    
    writeln!(Screen, "here after interrupt");

    // panic!();
    loop {}
    
}