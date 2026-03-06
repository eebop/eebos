#![no_std]
#![feature(allocator_api)]

extern crate alloc;

pub mod ports;
pub mod screen;

use core::{alloc::{Allocator, GlobalAlloc, Layout}, fmt::Debug, ops::{Deref, DerefMut}, panic::PanicInfo};

use alloc::boxed::Box;

use crate::screen::Screen;

use core::fmt::Write;

#[panic_handler]
fn panic<'a, 'b>(pi: &'a PanicInfo<'b>) -> ! {
    writeln!(Screen::new(), "panic!");
    writeln!(Screen::new(), "{}", pi);
    loop {}
}

struct EmptyAllocator;

unsafe impl GlobalAlloc for EmptyAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        panic!("Alloc not supported here!");
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        panic!("Alloc not supported here!");
    }
}

#[global_allocator]
static EMPTY_ALLOCATOR: EmptyAllocator = EmptyAllocator;

#[derive(Clone, Copy)]
#[repr(C, align(0x1000))]
pub struct Page([u8; 0x1000]);

impl Debug for Page {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Page").finish()
    }
}

impl Deref for Page {
    type Target = [u8; 0x1000];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Page {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Page {
	pub fn uninit_box<A: Allocator>(a: A) -> Box<Self, A> {
		unsafe { Box::new_uninit_in(a).assume_init() }
	}
    pub fn uninit_many<A: Allocator>(size: usize, a: A) -> Box<[Self], A> {
        unsafe { Box::new_uninit_slice_in(size, a).assume_init() }
    }
}

pub trait PageAligned<'a> {
    fn as_contiguous(self) -> &'a mut [u8];
}

impl<'a> PageAligned<'a> for &'a mut [Page] {
    fn as_contiguous(self) -> &'a mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(&raw mut self[0].0[0], 0x1000 * self.len())
        }
    }
}
