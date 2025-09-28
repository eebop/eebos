use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::arch::asm;

#[repr(C, align(0x1000))]
pub struct Page([u8; 0x1000]);

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
	pub fn uninit_box() -> Box<Self> {
		unsafe { Box::new_uninit().assume_init() }
	}
    pub fn uninit_many(size: usize) -> Box<[Self]> {
        unsafe { Box::new_uninit_slice(size).assume_init() }
    }
}

pub trait CoherentMultidemsionality<'a> {
    fn as_contiguous(self) -> &'a mut [u8];
}

impl<'a> CoherentMultidemsionality<'a> for &'a mut [Page] {
    fn as_contiguous(self) -> &'a mut [u8] {
        if let 0 = self.len() {
            &mut []
        } else {
            unsafe {
                core::slice::from_raw_parts_mut(&raw mut self[0].0[0], 0x1000 * self.len())
            }
        }
    }
}

pub struct Process {
    pub got_ptr: *mut [u8],
    pub _start: extern "C" fn() -> !,
    pub owned_data: Vec<Box<[Page]>>,
    pub stacks: Vec<Box<[Page]>>, // Unitialized
}

impl Process {
    pub fn new_stack(&mut self) {
        self.stacks.push(Page::uninit_many(4));
    }

    pub fn make_fncall(&mut self, _start: extern "C" fn() -> !) -> ! {
        // TODO: Initialization is more complex if you have argv that isn't empty
        self.new_stack();
        let size = self.stacks.last().unwrap().len() * 0x1000;
        self.stacks.last_mut().unwrap().last_mut().unwrap()[size-16..size].fill(0); // this sets up argv, argc, envp. all 0
        
        let ptr = &raw mut self.stacks.last_mut().unwrap()[size - 16];

        // ESP must point to the top of our stack
        // EDX should point to the atexit() function (not yet implemented)
        unsafe { asm!(
            "mov esp, {ptr}",
            "call {_start}",
            ptr = in(reg) &raw mut self.stacks.last_mut().unwrap().last_mut().unwrap()[size],
            _start = in(reg) _start,
            options(noreturn)
        )}
    }

}