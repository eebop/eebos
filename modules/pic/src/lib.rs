#![no_std]
#![no_main]

// writeln! generates these which is very annoying
#![allow(unused_must_use)]

// TODO: clean these up
#![allow(unused_imports)]

use core::{arch::asm, fmt::Write};
use dyshared::{bochsdbg, ports::{in8, io_wait, out8}, screen::Screen};


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

pub extern "C" fn test_clock() -> ! {
    // for i in 0..16 { 
    bochsdbg();
    enable(1);
    // }
    loop {
        // writeln!(Screen, "here");
        // send_EOI(1);
        // send_EOI(0);
    }
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

pub fn send_eoi(line: u8) {
    assert!(line < 16);
    if line >= 8 {
        out8(PicPort::Pic2Cmd as u16, PICEOI);
    }
    out8(PicPort::Pic1Cmd as u16, PICEOI);
}

