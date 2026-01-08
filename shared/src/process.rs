use alloc::alloc::Allocator;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::{Vec};
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::arch::asm;
use core::cell::RefCell;
use alloc::rc::Rc;
use core::pin::Pin;

unsafe extern "C" {
    static mut stored_sp: u32;
    static mut stack_top: u8;
}

#[repr(C, align(0x1000))]
pub struct Page([u8; 0x1000]);

impl !Unpin for Page {} // TODO: once paging is up, only a process's view is pinned

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

pub struct Process<A: Allocator + Clone, B: Allocator + Clone> {
    pub got_ptr: *mut [u8],
    pub _start: extern "C" fn() -> !,
    pub owned_data: Rc<RefCell<[Pin<Box<[Page], A>>]>, A>,
    pub stacks:     Rc<RefCell<Vec<Box<[Page], B>, B>>, B>
}

impl<A: Allocator + Clone, B: Allocator + Clone> Process<A, B> {
    pub fn empty(a: A) -> Self {
        Self {
            got_ptr: &mut [][..] as *mut [u8],
            _start: unsafe { core::mem::transmute(core::ptr::null::<()>()) },
            owned_data: Vec::new_in(a),
            stacks: None
        }
    }
    pub fn new_stack(&mut self, b: B) {
        if let None = self.stacks {
            self.stacks = Some(Vec::<Box<[Page], B>, B>::with_capacity_in(1, b.clone()));
        }

        self.stacks.as_mut().unwrap().push(Page::uninit_many(4, b))

    }

    pub fn make_fncall(&mut self, _start: extern "C" fn() -> !, stack_allocator: B) -> ! {

        // TODO: Initialization is more complex if you have argv that isn't empty
        self.new_stack(stack_allocator);

        let data = self.stacks.as_deref_mut().unwrap().last_mut().unwrap().as_contiguous();

        let size = data.len();
        assert!(size == 0x4000);
        data[size-16..size].fill(0); // this sets up argv, argc, envp. all 0

        // ESP must point to the top of our stack
        // EDX should point to the atexit() function (not yet implemented)
        unsafe { asm!(
            "mov esp, {ptr}",
            "call {_start}",
            ptr = in(reg) data.as_mut_ptr().add(size),
            _start = in(reg) _start,
            options(noreturn)
        )}
    }

}