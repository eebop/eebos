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

use core::prelude::*;
use core::ops::Range;
use core::{alloc::{GlobalAlloc, Layout}, fmt::{Write}, panic::PanicInfo};
use core::cell::RefCell;
use alloc::alloc::{Allocator, Global, alloc};
use alloc::collections::btree_map::{BTreeMap};
use alloc::vec::{self, Vec};
use elf::{self, segment::ProgramHeader};

use shared::process::{Page, Process};
use shared::screen::{self, Screen};
use shared::process::PageAligned;
use shared::{State, SysCallData, SysCallInternal};
use shared::std::{DummyAllocator, ManualOnceCell};

mod syscall;
mod elf;
use crate::syscall::STATE;

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

#[derive(Debug)]
struct RelocInfo {
    cmd: ProgramHeader,
    start: usize,
    length: usize
}

struct SimpleAllocator;

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align_needed = layout.align() - ((layout.align() - 1) & unsafe { DATAPTR.addr()});

        unsafe {
            DATAPTR = DATAPTR.add(align_needed);
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

fn reinterpret_slice<T, U>(i: &[T]) -> Option<&[U]> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return None;
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *const U;
        Some(slice::from_raw_parts(ptr, newsize))
    }
}

fn reinterpret_slice_mut<T, U>(i: &mut [T]) -> Option<&mut [U]> {
    let size = i.len() * size_of::<T>();
    if size % size_of::<U>() != 0 {
        return None;
    }
    let newsize = size / size_of::<U>();
    unsafe {
        let ptr = i.as_ptr() as *mut U;
        Some(slice::from_raw_parts_mut(ptr, newsize))
    }
}

fn relocate_symbol(symbol: u64, relocations: &Vec<RelocInfo>) -> (usize, u64) {
    for reloc in relocations.iter().enumerate() {
        if reloc.1.cmd.p_vaddr <= symbol && symbol < reloc.1.cmd.p_vaddr + reloc.1.cmd.p_memsz {
            // Match
            let index = symbol - reloc.1.cmd.p_vaddr;
            return (reloc.0, index);
        }
    }
    panic!("Unable to relocate symbol located at {}", symbol);
}

fn relocate_as_ptr(symbol: usize, relocations: &Vec<RelocInfo>) -> usize {
    let (index, address) = relocate_symbol(symbol as u64, relocations);
    relocations[index].start + address as usize
}

fn as_fn_ptr<T>(ptr: usize) -> fn() -> T {
    unsafe {
        core::mem::transmute(ptr)
    }
}

fn load_syscall(data: SysCallData, state: &State) {
    let inner: &str = data.receive_abi();
    let out = string_elf(inner);
    writeln!(Screen::new(), "sending... {out:#?}");
    data.send_abi(out);
}

// Called by other process
// TODO: with the switch to paging this'll be more difficult
fn kmalloc(data: SysCallData, state: &State) {
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
        // Every instance if ManualOnceCell must be initialized here
        init_elf_data();
    }

    let mut s = Screen { line: 0, row: 0};

    s.clear_screen();

    

    let code: &mut [u8] = unsafe { slice::from_mut_ptr_range(elf_data!(start_process)) };
    let mut pic = load_elf::<Global>(code);
        
    // STATE.processes.borrow_mut().push(pic);

    // STATE.currentProcess.replace(Some(0));


    {    
        let mut ptr = STATE.interrupts.borrow_mut();
        ptr[0x30] = shared::Syscall::Request(syscall::submit_syscall);
        ptr[0x50] = shared::Syscall::Request(kmalloc);
        ptr[0xfe] = shared::Syscall::Request(load_syscall);
    };

    // // SAFETY: we will not aquire another borrow of processes
    // // We must do this because making a syscall doesn't drop anything
    // // Meaning if we did .borrow_mut() then after the syscall it'd be unusable
    // let p = unsafe { STATE.processes.as_ptr().as_mut_unchecked() };

    let ptr = pic._start;

    pic.make_fncall(ptr, Global);
}

