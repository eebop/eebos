#![no_std]
#![feature(pointer_is_aligned_to)]
#![feature(allocator_api)]

extern crate alloc;

use core::arch::{asm, naked_asm};

use dyshared::{self, CAllocator, bochsdbg, screen::Screen};
use paging;

use core::fmt::Write;

mod tasks;
mod idt;

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

    bochsdbg();
    unsafe {
        asm!(
            "int 3"
        )
    }
    
    writeln!(Screen, "here after interrupt");

    loop {}
    
}