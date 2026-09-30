use core::fmt;
use core::fmt::Write;
use core::slice;

use crate::ports::out8;

static mut LINE: usize = 0;
static mut ROW: usize = 0;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum VgaColor {
	Black = 0,
	Blue = 1,
	Green = 2,
	Cyan = 3,
	Red = 4,
	Magenta = 5,
	Brown = 6,
	LightGrey = 7,
	DarkGrey = 8,
	LightBlue = 9,
	LightGreen = 10,
	LightCyan = 11,
	LightRed = 12,
	LightMagenta = 13,
	LightBrown = 14,
	White = 15,
}

impl VgaColor {
    const fn make_color(fg: VgaColor, bg: VgaColor) -> u8 {
        return ((fg as u8) << 0)
             + ((bg as u8) << 4)
    }
}

const fn make_char(color: u8, char: u8) -> u16 {
    return ((color as u16) << 8) + char as u16;
}


pub struct Screen;

impl Screen {
    pub unsafe fn coord() -> usize {
        return unsafe { LINE } * 80 + unsafe { ROW };
    }

    pub fn write_byte(&mut self, c: u8) {
        let color: u8 = VgaColor::make_color(VgaColor::LightGrey, VgaColor::Black);
        let screen: &mut [u16] = unsafe {
            slice::from_raw_parts_mut(0xB8000 as *mut u16, 25 * 80)
        };

        unsafe {
            if c == b'\n' {
                screen[Screen::coord()..(LINE * 80 + 80)].fill(make_char(color, b' '));
                ROW = 0;
                LINE += 1;
            } else {
                screen[Screen::coord()] = make_char(color, c);

                ROW += 1;
                if ROW == 80 {
                    ROW = 0;
                    LINE += 1;
                }
            }
            if LINE == 25 {
                LINE = 24;
                screen.copy_within(80..(80*25), 0);
                screen[(80*24)..(80*25)].fill(make_char(color, b' '));
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
