
#![no_std]
#![no_main]

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

struct Screen {
    line: usize,
    row: usize
}

impl Screen {
    pub fn coord(&self) -> usize {
        return self.line * 80 + self.row;
    }

    pub fn write_byte(&mut self, c: u8) {
        let screen: &mut [u16] = unsafe {
            slice::from_raw_parts_mut(0xB8000 as *mut u16, 25 * 80)
        };

        if c == b'\n' {
            while self.row != 80 {
                screen[self.coord()] = (screen[self.coord()] & 0xFF00) | (b' ' as u16);
                self.row += 1;
            }
            self.row = 0;
            self.line += 1;
            if self.line == 25 {
                self.line = 0;
            }
            return;
        }

        screen[self.coord()] = (screen[self.coord()] & 0xFF00) | (c as u16);

        self.row += 1;
        if self.row == 80 {
            self.row = 0;
            self.line += 1;
            if self.line == 25 {
                self.line = 0;
            }
        }
    }

    pub fn clear_screen(&mut self) {
        for _ in 0..(25 * 80) {
            self.write_byte(b' ');
        }
    }

}

impl fmt::Write for Screen {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.as_bytes() {
            self.write_byte(*byte);
        }
        Ok(())
    }
}

static mut x: u8 = 6;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> u32 {
    let mut s = Screen { line: 0, row: 0};
    // s.clear_screen();
    writeln!(&mut s, "====Here!====");
    unsafe {asm!("int 0")};
    return 0;
}
