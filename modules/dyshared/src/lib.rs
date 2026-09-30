#![no_std]
#![feature(allocator_api)]
#![feature(trait_alias)]

// writeln! generates these which is very annoying
#![allow(unused_must_use)]

// TODO: clean these up
#![allow(unused_imports)]

extern crate alloc;

pub mod ports;
pub mod screen;

use core::{alloc::{Allocator, GlobalAlloc, Layout}, fmt::Debug, num::NonZero, ops::{Deref, DerefMut}, panic::PanicInfo, ptr::{self, NonNull}};

use alloc::{alloc::{AllocError, AllocatorClone, Global}, boxed::Box};

use crate::screen::Screen;

use core::fmt::Write;

#[unsafe(no_mangle)]
#[inline(never)]
fn rmpanic_mark() {

}

#[panic_handler]
fn panic<'a, 'b>(pi: &'a PanicInfo<'b>) -> ! {
    writeln!(Screen, "{}", pi);
    rmpanic_mark();
    loop {}
}

pub static mut HEAP_PTR: NonNull<u8> = NonNull::dangling();

#[derive(Clone, Copy, Debug)]
pub struct SimpleAllocator;

unsafe impl Allocator for SimpleAllocator {
        
    fn allocate(&self, layout: Layout) -> Result<ptr::NonNull<[u8]>, AllocError> {
        let mask = layout.align() - 1;
        let out = unsafe {
            HEAP_PTR = HEAP_PTR.add(mask).map_addr(|addr| NonZero::new(addr.get() & !(mask)).unwrap());
            let out = HEAP_PTR;

            HEAP_PTR = HEAP_PTR.add(layout.size());
            out
        };
        Ok(NonNull::slice_from_raw_parts(out, layout.size()))
    }
    
    unsafe fn deallocate(&self, _ptr: ptr::NonNull<u8>, _layout: Layout) {
    }
}

unsafe impl AllocatorClone for SimpleAllocator {}


unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocate(layout).unwrap().as_ptr() as *mut u8
    }
    
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
    }
}


pub trait CAllocator = AllocatorClone + Debug;

#[global_allocator]
static EMPTY_ALLOCATOR: SimpleAllocator = SimpleAllocator;

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
    pub fn try_uninit_box<A: Allocator>(a: A) -> Result<Box<Self, A>, AllocError> {
        Ok(unsafe { Box::try_new_uninit_in(a)?.assume_init() })
    }

    pub fn uninit_box<A: Allocator>(a: A) -> Box<Self, A> {
        Self::try_uninit_box(a).unwrap()
    }

    pub fn try_uninit_many<A: Allocator>(size: usize, a: A) -> Result<Box<[Self], A>, AllocError> {
        Ok(unsafe { Box::try_new_uninit_slice_in(size, a)?.assume_init()})
    }

    pub fn uninit_many<A: Allocator>(size: usize, a: A) -> Box<[Self], A> {
        Self::try_uninit_many(size, a).unwrap()
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

pub fn bochsdbg() {
	unsafe { core::arch::asm!("xchg bx, bx") };
}