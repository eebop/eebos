#![no_std]
#![no_main]

// writeln! generates these which is very annoying
#![allow(unused_must_use)]

// TODO: clean these up
#![allow(unused_imports)]

#![feature(allocator_api)]
#![feature(ptr_mask)]
#![feature(macro_metavar_expr_concat)]

#![allow(internal_features)]
#![feature(rustc_attrs)]
#![feature(slice_from_ptr_range)]

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

use core::{alloc::{GlobalAlloc, Layout}, arch::asm, fmt::Write, panic::PanicInfo, mem::transmute};
use alloc::{alloc::{Global, alloc}, collections::btree_map::BTreeMap};
use alloc::vec::Vec;

use shared::{bochsdbg, screen::Screen, SysCallInternal};
use shared::SysCallData;

mod elf;

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

fn make_fncall(ptr: extern "C" fn(*mut u8, *const BTreeMap<&str, &[u8]>) -> !, memptr: *mut u8, elfdata: *const BTreeMap<&str, &[u8]>) -> ! {
    ptr(memptr, elfdata)
}

#[unsafe(no_mangle)]
pub extern "C" fn rustmain(mem: *mut u8) {
    // This code must be performed as soon as we get control
    unsafe {
        DATAPTR = mem;
        // Every instance of ManualOnceCell must be initialized here
        elf::init_elf_data();
    }

    let proc = elf::load_mod("libtest_mod.so");
    writeln!(Screen::new(), "now here in rustmain");
    let ptr = &proc.symbols["start"];
    let ptr = ptr.1.relocate_ptr(ptr.0.st_value as u32);

    // writeln!(Screen::new(), "args are: {:?}, {:?}", unsafe { DATAPTR } as usize, &raw const *elf::ELF_DATA.get() as usize);
    // make_fncall(ptr as usize, unsafe { DATAPTR } as usize, &raw const *elf::ELF_DATA.get() as usize);
    // unsafe { core::mem::transmute::<_, extern "C" fn(usize, usize) -> !>(ptr)(unsafe { DATAPTR } as usize, &raw const *elf::ELF_DATA.get() as usize) };
    make_fncall(unsafe { core::mem::transmute(ptr) }, unsafe { DATAPTR }, &raw const *elf::ELF_DATA.get())
}




#[unsafe(no_mangle)]
pub extern "C" fn isr_handler(regs: *mut SysCallInternal) {
    panic!("{:?}", unsafe {*regs});
}