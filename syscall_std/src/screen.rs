use core::fmt;
use core::slice;

pub struct Screen {
    pub line: usize,
    pub row: usize
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
