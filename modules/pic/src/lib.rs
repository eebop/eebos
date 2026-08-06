#![no_std]
#![no_main]

#![feature(never_type)]

use core::panic::PanicInfo;

use core::*;

use core::fmt::Write;

use core::alloc::GlobalAlloc;

use dyshared::ports::{io_wait, in8, out8};
use dyshared::screen::Screen;


#[derive(Clone, Copy)]
enum PicPort {
    Pic1Cmd = 0x20,
    Pic1Data = 0x21,
    Pic2Cmd = 0xA0,
    Pic2Data = 0xA1
}

const PICEOI: u8 = 0x20;

fn get_state() -> u16 {
    let pic1: u16 = in8(PicPort::Pic1Data as u16).into();
    let pic2: u16 = in8(PicPort::Pic2Data as u16).into();
    return (pic2 << 8) + pic1;
}

pub fn enable(line: u8) {
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

fn sendEOI(line: u8) {
    assert!(line < 16);
    if line >= 8 {
        out8(PicPort::Pic2Cmd as u16, PICEOI);
    }
    out8(PicPort::Pic1Cmd as u16, PICEOI);
}

