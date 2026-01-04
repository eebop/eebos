use core::{cell::SyncUnsafeCell, mem::MaybeUninit};

use alloc::alloc::Allocator;

pub struct ManualOnceCell<T> {
    inner: SyncUnsafeCell<MaybeUninit<T>>
}

impl<T> ManualOnceCell<T> {
    pub const fn new() -> Self {
        Self { inner: SyncUnsafeCell::new(MaybeUninit::uninit()) }
    }
    pub fn get(&self) -> &T {
        unsafe { self.inner.get().as_ref_unchecked().assume_init_ref() }
    }

    pub unsafe fn init(&self, val: T) {
        unsafe {
            *self.inner.get() = MaybeUninit::new(val);
        }
    }
}

#[derive(Clone, Copy)]
pub struct DummyAllocator;

unsafe impl Allocator for DummyAllocator {
    fn allocate(&self, _: core::alloc::Layout) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        panic!("allocate() attempt in dummy allocator");
    }

    unsafe fn deallocate(&self, _: core::ptr::NonNull<u8>, _: core::alloc::Layout) {
        panic!("deallocate() attempt in dummy allocator");
    }
}