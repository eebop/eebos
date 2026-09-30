use core::{cell::UnsafeCell, marker::PhantomData, ops::Deref};

// Represents a lifetime for which the system will be in single-threaded mode
// Also stores what SingleThreaded data we've looked at before to prevent multiple acquires
pub struct SingleThreadToken<'a> {
    _this: PhantomData<&'a ()>
}

impl<'a> SingleThreadToken<'a> {

    /// Create a guarentee of single-threadedness
    /// Requirements: No other thread
    /// is concurently executing
    /// There must be at least an Acquire/Release
    /// relation between the deletion of a previous token
    /// and the creation of this one
    /// Note that "deletion" may include pausing a token
    /// (in that case unpausing is a creation event)
    pub unsafe fn new() -> Self {
        Self {
            _this: PhantomData
        }
    }

    pub fn reborrow(&mut self) -> SingleThreadToken<'_> {
        SingleThreadToken { _this: PhantomData }
    }
}

pub struct SingleThreaded<T> {
    inner: UnsafeCell<T>
}

impl<T> SingleThreaded<T> {
    pub const fn new(val: T) -> Self {
        Self {
            inner: UnsafeCell::new(val)
        }
    }

    /// Acquire with lifetime proof
    pub fn get<'a>(&'a self, proof: &'a mut SingleThreadToken<'a>) -> &'a T {
        unsafe { self.inner.as_mut_unchecked() }
    }
}


unsafe impl<T> Send for SingleThreaded<T> where T: Send {}
unsafe impl<T> Sync for SingleThreaded<T> where T: Send {}