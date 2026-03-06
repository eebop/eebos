use alloc::alloc::Allocator;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::{Vec};
use core::ops::{Deref, DerefMut, Index, IndexMut, Range};
use core::arch::asm;
use core::cell::RefCell;
use alloc::rc::Rc;
use core::pin::Pin;
use core::fmt::Debug;


unsafe extern "C" {
    static mut stored_sp: u32;
    static mut stack_top: u8;
}

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

#[derive(Debug, Clone)]
pub struct Process<A: Allocator + Clone, B: Allocator + Clone> {
    pub got_ptr: *mut [u32],
    pub _start: extern "C" fn() -> !,
    pub owned_data: Rc<RefCell<[Box<[Page], A>]>, A>,
    pub stacks: Vec<Rc<RefCell<Box<[Page], B>>, B>, B>
}

impl<A: Allocator + Clone, B: Allocator + Clone> Process<A, B> {
    pub fn new(got_ptr: *mut [u32], _start: extern "C" fn() -> !, owned_data: Rc<RefCell<[Box<[Page], A>]>, A>, b: B) -> Self {
        Self {
            got_ptr: got_ptr,
            _start: _start,
            owned_data: owned_data,
            stacks: Vec::new_in(b)
        }
    }
    pub fn new_stack(&mut self, alloc: B) -> Rc<RefCell<Box<[Page], B>>, B> {

        let data = Rc::new_in(RefCell::new(Page::uninit_many(4, alloc.clone())), alloc.clone());

        self.stacks.push(data.clone());
        
        data
    }

    pub fn try_own_none<C: Allocator + Clone>(self, alloc: C) -> Result<Process<A, C>, Process<A, B>> {
        let 0 = self.stacks.len() else {
            return Err(self);
        };
        Ok(Process { got_ptr: self.got_ptr, _start: self._start, owned_data: self.owned_data, stacks: Vec::new_in(alloc) })
    }

    pub fn make_fncall(&mut self, _start: extern "C" fn() -> !, stack_allocator: B) -> ! {
        let ptr = {
            // TODO: Initialization is more complex if you have argv that isn't empty
            let mut stack = self.new_stack(stack_allocator);
            let mut borrow = stack.borrow_mut();
            let mut data = borrow.as_contiguous();

            let size = data.len();
            assert!(size == 0x4000);
            data[size-16..size].fill(0); // this sets up argv, argc, envp. all 0
            unsafe { data.as_mut_ptr().add(size - 16) }
        };


        // ESP must point to the top of our stack
        // EDX should point to the atexit() function (not yet implemented)
        unsafe { asm!(
            "mov esp, {ptr}",
            "call {_start}",
            ptr = in(reg) ptr,
            _start = in(reg) _start,
            options(noreturn)
        )}
    }

}