#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[path = "../ports_h.rs"]
mod ports_h;

use ports_h::{inb, outb};

#[path = "../stdutils_h.rs"]
mod stdutils;

use stdutils::printf;

#[path = "../pic_h.rs"]
mod pic;

use pic::IRQ_clear_mask;


// use ports_h::{inb, outb};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

enum PS2Token {
    MAIN,
    AUX
}

enum PS2Port {
    CMD,
    DATA
}

fn input(port: PS2Port) -> u8 {
    unsafe { inb(match port {
        PS2Port::CMD => 0x64,
        PS2Port::DATA => 0x60
    }) }
}

fn output(port: PS2Port, value: u8) {
    unsafe { outb(match port {
        PS2Port::CMD => 0x64,
        PS2Port::DATA => 0x60
    }, value) }
}

fn ps2_write(port: PS2Port, data: u8) -> Result<(), ()> {
    let mut tries = 1000;
    let mut status: u8;
    loop {
        tries -= 1;
        if tries == 0 {return Result::Err(())}
        status = input(PS2Port::CMD);
        if (status & 0x2) == 0 {break}
    }
    output(port, data);
    return Result::Ok(());
}

fn ps2_read(port: PS2Port) -> Result<u8, ()> {
    let mut tries = 1000;
    let mut status: u8;
    loop {
        tries -= 1;
        if tries == 0 {return Result::Err(())}
        status = input(PS2Port::CMD);
        if (status & 0x1) == 1 {break}
    }
    return Result::Ok(input(port));
}

#[no_mangle]
pub extern "C" fn mouse_in() {
    let press = ps2_read(PS2Port::DATA);
}

#[no_mangle]
pub extern "C" fn ps2_init()  {
    _ps2_init();
}

fn _ps2_init() -> Result<(), ()> {
    ps2_write(PS2Port::CMD, 0x20);
    let mut ccb: u8 = ps2_read(PS2Port::DATA)?;
    ccb |= 0x2;
    ps2_write(PS2Port::CMD, 0x60)?;
    ps2_write(PS2Port::DATA, ccb)?;

    ps2_write(PS2Port::CMD, 0xA8)?;

    unsafe {
        IRQ_clear_mask(1);
        IRQ_clear_mask(12);
    }
    Ok(())
}