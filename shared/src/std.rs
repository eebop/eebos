use core::{alloc::GlobalAlloc, cell::SyncUnsafeCell, mem::MaybeUninit};

use alloc::alloc::Allocator;
use core::alloc::Layout;
use core::fmt::Write;

use crate::{make_syscall, screen::Screen};

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

#[derive(Debug, Clone, Copy)]
pub struct DummyAllocator;

unsafe impl Allocator for DummyAllocator {
    fn allocate(&self, _: core::alloc::Layout) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        panic!("allocate() attempt in dummy allocator");
    }

    unsafe fn deallocate(&self, _: core::ptr::NonNull<u8>, _: core::alloc::Layout) {
        panic!("deallocate() attempt in dummy allocator");
    }
}

pub struct SimpleAllocator;

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        writeln!(Screen::new(), "making alloc attempt...");
        make_syscall::<Layout, *mut u8, 0x50>(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        // No dealloc in kernel yet, so just pass
    }
}

struct EmptyAllocator;

unsafe impl GlobalAlloc for EmptyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        panic!("Alloc not supported here!");
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        panic!("Alloc not supported here!");
    }
}