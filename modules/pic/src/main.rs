#![no_std]
#![no_main]


use shared::screen::Screen;

use core::panic::PanicInfo;

use core::*;

use core::fmt::Write;

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
fn panic<'a, 'b>(_: &'a PanicInfo<'b>) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn __libc_start_main() {
    main();
}

#[unsafe(no_mangle)]
pub extern "C" fn main() {
    make_syscall::<NewSysCall, (), 0x30>(NewSysCall { channel: 0x20, ptr: clock });
    enable(0);
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

pub fn clock(cmd: &mut SysCallData, state: &mut State) {
	writeln!(state.screen, "Clock!");
	sendEOI(0);
}


fn sendEOI(line: u8) {
    assert!(line < 16);
    if line >= 8 {
        out8(PicPort::Pic2Cmd as u16, PICEOI);
    }
    out8(PicPort::Pic1Cmd as u16, PICEOI);
}

