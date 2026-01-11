use core::fmt;
use core::slice;

use crate::ports::out8;

pub struct Screen {
    pub line: usize,
    pub row: usize
}

impl Screen {
    pub fn new() -> Self {
        Self { line: 0, row: 0 }
    }

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
        } else {

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
        out8(0xe9, c);


    }

    pub fn clear_screen(&mut self) {
        let screen: &mut [u16] = unsafe {
            slice::from_raw_parts_mut(0xB8000 as *mut u16, 25 * 80)
        };
        // TODO: if there's color, we need to reset it
        screen.fill((screen[0] & 0xFF00) | (b' ' as u16));
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
