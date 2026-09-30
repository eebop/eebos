use core::{arch::asm, cell::{Cell, UnsafeCell}, ops::{Deref, DerefMut}, pin::Pin, sync::atomic::{AtomicBool, Ordering::{AcqRel, Acquire, Release}}, task::{Context, Poll, Waker}};

use alloc::{boxed::Box, collections::VecDeque, rc::{Rc, Weak}, vec::Vec};

#[repr(transparent)]
struct Lock {
    // True => unlocked
    // False => locked
    inner: AtomicBool
}

impl Lock {
    fn new() -> Self {
        Self { inner: AtomicBool::new(true) }
    }
    
    /// true means acquired lock
    /// false means already locked
    fn try_lock(&self) -> bool {
        // todo: hopefully this is the right ordering
        self.inner.swap(false, AcqRel)
    }

    fn is_unlocked(&self) -> bool {
        self.inner.load(Acquire)
    }

    fn release(&self) {
        self.inner.store(true, Release);
    }
}

struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.mutex.inner.as_ref_unchecked() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.mutex.inner.as_mut_unchecked() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.get_view_lock();

        self.mutex.data_lock.release();


        // Any of the Waiters could be dead (Future dropped)
        // Simplest thing to do is just wake all the futures
        // TODO: maybe we can get a stronger garentee and only wake some of them?

        // SAFETY: we have the view_lock
        let waiters = unsafe { self.mutex.waiting.replace(Vec::new()) };

        self.mutex.view_lock.release();

        for waiter in waiters {
            // There is always only one, so this should always work
            let obj = Rc::try_unwrap(waiter).unwrap_or_else(|_| panic!());
            obj.into_inner().wake();
        }

    }
}

struct Mutex<T> {
    // Locks a view of mutex to prevent races for queue
    view_lock: Lock,

    // Ownership of data
    data_lock: Lock,
    inner: UnsafeCell<T>,

    waiting: UnsafeCell<Vec<Rc<Cell<Waker>>>>
}

impl<T> Mutex<T> {
    fn new(item: T) -> Self {
        Self {
            view_lock: Lock::new(),
            data_lock: Lock::new(),
            inner: UnsafeCell::new(item),

            waiting: UnsafeCell::new(Vec::new()),
        }
    }

    fn get_view_lock(&self) {
        while !self.view_lock.try_lock() {
            core::hint::spin_loop();
        }
    }

    fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.get_view_lock();
        let res = self.data_lock.try_lock();

        self.view_lock.release();
        if res {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }

    fn lock<'a>(&'a self) -> impl Future<Output = MutexGuard<'a, T>> {
        
        struct MutexFuture<'f, T> {
            inner: &'f Mutex<T>,
            future: Weak<Cell<Waker>>
        }

        impl<'f, T> Future for MutexFuture<'f, T> {
            type Output = MutexGuard<'f, T>;
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<MutexGuard<'f, T>> {

                self.inner.get_view_lock();
                
                if self.inner.data_lock.try_lock() {
                    self.inner.view_lock.release();
                    return Poll::Ready(MutexGuard { mutex: self.inner })
                }
                
                if let Some(waker) = self.future.upgrade() {
                    // Still in line, update future
                    waker.set(cx.waker().clone());
                    self.inner.view_lock.release();
                    return Poll::Pending
                }

                let data = Rc::new(Cell::new(cx.waker().clone()));

                self.future = Rc::downgrade(&data);

                // SAFETY: We own the queue's lock (view_lock)
                let queue = unsafe { self.inner.waiting.as_mut_unchecked() };
                queue.push(data);

                self.inner.view_lock.release();

                return Poll::Pending;
            }
        }
        return MutexFuture {inner: self, future: Weak::new()}
    }
}

unsafe impl<T> Send for Mutex<T> where T: Send {}
unsafe impl<T> Sync for Mutex<T> where T: Send {}