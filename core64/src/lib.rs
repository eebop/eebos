#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(ptr_mask)]
#![feature(alloc_error_handler)]
#![feature(macro_metavar_expr_concat)]

#![allow(internal_features)]
#![feature(rustc_attrs)]
#![feature(ptr_as_ref_unchecked)]
#![feature(slice_from_ptr_range)]
#![feature(const_slice_from_ptr_range)]
#![feature(sync_unsafe_cell)]
#![feature(generic_const_exprs)]

// Todo: add checks for all the writeln!s
// #![allow(unused)]

// This symbol is required for an allocator to work with --emit obj in no_std
// My understanding is that it "tells" the compiler that you know what you're doing
#[rustc_std_internal_symbol]
fn __rust_no_alloc_shim_is_unstable_v2() {}

// I have no idea why I need this and why #[alloc_error_handler] doesn't work

#[rustc_std_internal_symbol]
fn __rust_alloc_error_handler(_: core::alloc::Layout) -> ! {
    panic!("memory allocation failed");
}

#[macro_use]
extern crate alloc;

use core::{alloc::{GlobalAlloc, Layout}, fmt::{Write}, panic::PanicInfo};
use alloc::alloc::{Global, alloc};
use alloc::vec::Vec;

use shared::{bochsdbg, screen::Screen, SysCallInternal};
use shared::SysCallData;

// mod syscall;
mod elf;
// use syscall::STATE;

#[panic_handler]
fn panic<'a, 'b>(info: &'a PanicInfo<'b>) -> ! {
    let mut s = Screen {line: 0, row: 0};
    writeln!(&mut s, "{}", info);
    loop {}
}

// The inits aren't actually called, so global = 0 does nothing
// Initialization must be done in rustmain()

// Passes data to the allocator from main()
static mut DATAPTR: *mut u8 = core::ptr::null_mut();

struct SimpleAllocator;

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mask = layout.align() - 1;
        unsafe {
            DATAPTR = DATAPTR.add(mask).mask(!mask);
            let out = DATAPTR;

            DATAPTR = DATAPTR.add(layout.size());
            out
        }
    }
    
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // pass
    }
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator;

// Called by other process
// TODO: with the switch to paging this'll be more difficult
fn kmalloc(mut data: SysCallData) {
    let inner: Layout = data.receive_abi();
    writeln!(Screen::new(), "allocating... {inner:?}");
    let out = unsafe { alloc(inner) };
    data.send_abi(out);
}

#[unsafe(no_mangle)]
pub extern "C" fn rustmain(mem: *mut u8) {
    // This code must be performed as soon as we get control
    unsafe {
        DATAPTR = mem;
        // Every instance of ManualOnceCell must be initialized here
        elf::init_elf_data();

    }
    let mut s = Screen::new();

    s.clear_screen();


    let mut proc = elf::load_mod("libtest_mod.so");
    writeln!(Screen::new(), "HERE");
    // for ptr in proc.init_fns {
    //     writeln!(Screen::new(), "here, calling {ptr}");
    //     let ptr: extern "C" fn() = unsafe { core::mem::transmute(ptr) };
    //     ptr();
    // }
    let ptr = &proc.symbols["main"];
    let ptr = ptr.1.relocate_ptr(ptr.0.st_value as u32);
    // for x in 0..10 {
    //     write!(Screen::new(), "{:x?} ", unsafe{ *ptr.add(x) });

    // }
    let ptr: extern "C" fn(*mut u8) = unsafe { core::mem::transmute(ptr) };
    ptr(unsafe { DATAPTR });
    // unsafe {
    //     for i in 0..10 {
    //         write!(Screen::new(), "{:x?} ", *ptr.add(i));
    //     }
    // }
    loop {}
    // STATE.processes.borrow_mut().push(pic);

    // STATE.currentProcess.replace(Some(0));


    // {    
    //     let mut ptr = STATE.interrupts.borrow_mut();
    //     // ptr[0x30] = shared::Syscall::Request(syscall::submit_syscall);

    //     ptr[0x20] = shared::Syscall::Request(syscall::api_request);
    //     ptr[0x21] = shared::Syscall::Request(syscall::api_return);
    //     ptr[0x22] = shared::Syscall::Request(syscall::submit_interface);

    //     ptr[0x50] = shared::Syscall::Request(kmalloc);
    //     ptr[0xfe] = shared::Syscall::Request(load_syscall);
    // };

    // let ptr = proc._start;

    // proc.make_fncall(ptr, Global);
}

#[unsafe(no_mangle)]
pub extern "C" fn isr_handler(regs: *mut SysCallInternal) {
    panic!("{:?}", unsafe {*regs});
}