#![no_std]
#![no_main]

use core::panic::PanicInfo;

use core::*;

use core::fmt::Write;

use core::alloc::GlobalAlloc;

use shared::ports::{io_wait, in8, out8};
use shared::{make_syscall, NewSysCall, State, SysCallData};


#[derive(Clone, Copy)]
enum PicPort {
    Pic1Cmd = 0x20,
    Pic1Data = 0x21,
    Pic2Cmd = 0xA0,
    Pic2Data = 0xA1
}

const PICEOI: u8 = 0x20;

#[panic_handler]
fn panic<'a, 'b>(p: &'a PanicInfo<'b>) -> ! {
    let mut s = shared::screen::Screen {line: 0, row: 0};
    writeln!(&mut s, "{}", p);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn __libc_start_main() {
    main();
}

struct EmptyAllocator;

unsafe impl GlobalAlloc for EmptyAllocator {
    unsafe fn alloc(&self, layout: alloc::Layout) -> *mut u8 {
        panic!("Alloc not supported here!");
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: alloc::Layout) {
        panic!("Alloc not supported here!");
    }
}

#[global_allocator]
static EMPTY_ALLOCATOR: EmptyAllocator = EmptyAllocator;


#[unsafe(no_mangle)]
pub extern "C" fn main() {
    let mut s = shared::screen::Screen {line: 0, row: 0};
    // let data = NewSysCall { channel: 0x20, ptr:  clock };
    // make_syscall::<NewSysCall, (), 0x30>(data);
    s.clear_screen();
    writeln!(&mut s, "here now");
    // enable(0);
    loop {}
}

fn get_state() -> u16 {
    let pic1: u16 = in8(PicPort::Pic1Data as u16).into();
    let pic2: u16 = in8(PicPort::Pic2Data as u16).into();
    return (pic2 << 8) + pic1;
}

fn enable(line: u8) {
    assert!(line < 16);

    let port = if line & 8 == 8 {
        PicPort::Pic2Data
    } else {
        PicPort::Pic1Data
    };

    io_wait();
    let mut curr = in8(port as u16);
    curr &= !(1 << (line & 7));

    out8(port as u16, curr);
}

pub fn clock(cmd: SysCallData, state: &State) {
    state.screen.borrow_mut().clear_screen();
	writeln!(state.screen.borrow_mut(), "Clock!");
	sendEOI(0);
}


fn sendEOI(line: u8) {
    assert!(line < 16);
    if line >= 8 {
        out8(PicPort::Pic2Cmd as u16, PICEOI);
    }
    out8(PicPort::Pic1Cmd as u16, PICEOI);
}

