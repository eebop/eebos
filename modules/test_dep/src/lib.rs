#![no_std]
#![no_main]

// writeln! generates these which is very annoying
#![allow(unused_must_use)]

// TODO: clean these up
#![allow(unused_imports)]


extern crate dyshared;

use dyshared::screen::Screen;


use core::fmt::Write;

#[unsafe(no_mangle)]
pub extern "C" fn test() -> u32 {
    let mut s = Screen;
    writeln!(&mut s, "====Here!====");

    // shared::make_syscall::<u32, u32, 0xff>(0x1f1f);

    return 0;
}
