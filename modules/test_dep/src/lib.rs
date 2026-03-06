#![no_std]
#![no_main]

extern crate dyshared;

use dyshared::screen::Screen;


use core::fmt::Write;

#[unsafe(no_mangle)]
pub extern "C" fn test() -> u32 {
    let mut s = Screen { line: 0, row: 0};
    s.clear_screen();
    writeln!(&mut s, "====Here!====");

    // shared::make_syscall::<u32, u32, 0xff>(0x1f1f);

    return 0;
}
