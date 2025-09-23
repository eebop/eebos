#![no_std]
#![no_main]


use shared::screen::Screen;

use core::panic::PanicInfo;

use core::arch::asm;
use core::*;
use core::fmt::Write;

#[panic_handler]
fn panic<'a, 'b>(_: &'a PanicInfo<'b>) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn __libc_start_main() {
    main();
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> u32 {
    let mut s = Screen { line: 0, row: 0};
    // s.clear_screen();
    writeln!(&mut s, "====Here!====");

    shared::make_syscall::<u32, u32, 0xff>(0x1f1f);

    return 0;
}
