extern crate dyshared;

use alloc::{alloc::{AllocError, Allocator, Global}, boxed::Box, rc::Rc};

// use rangemap::{RangeInclusiveMap, RangeMap, StepFns};
use dyshared::{Page, screen::Screen};
use static_assertions::const_assert;
use core::{alloc::Layout, cell::{Cell, RefCell}, fmt::{Debug, Formatter}, num::NonZero, pin::Pin, ptr::NonNull};
use crate::{AllocationStrategy, PageMap, PageToken, PageType, Permission};
use core::arch::asm;
use core::fmt::Write;

#[derive(Clone, Copy, Debug)]
pub enum PageError {
    PageExists
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct Page32Element {
    inner: Option<NonZero<u32>>
}

impl Page32Element {
    const fn new_empty() -> Self {
        Self { inner: None }
    }

    fn new_exists(ptr: *const Page, global: bool, pat_or_sz: bool, dirty: bool, accessed: bool, cache_disable: bool, write_through: bool, allow_user: bool, allow_write: bool) -> Self {
        let global = global as u32;
        let pat_or_sz = pat_or_sz as u32;
        let dirty = dirty as u32;
        let accessed = accessed as u32;
        let cache_disable = cache_disable as u32;
        let write_through = write_through as u32;
        let allow_user = allow_user as u32;
        let allow_write = allow_write as u32;

        assert!(ptr.is_aligned());
        let val = ptr as u32;
        let val = val
            + (global << 8)
            + (pat_or_sz << 7)
            + (dirty << 6)
            + (accessed << 5)
            + (cache_disable << 4)
            + (write_through << 3)
            + (allow_user << 2)
            + (allow_write << 1)
            + 1;
        
        Self { inner: NonZero::new(val) }
    }

    fn get_ptr(&self) -> Option<*mut Page> {
        let Some(inner) = self.inner else {
            return None;
        };
        Some((inner.get() & 0xFFFFF000) as *mut Page)
    }

    fn is_empty(self) -> bool {
        self.inner.is_none()
    }
}

const impl Default for Page32Element {
    fn default() -> Self {
        Page32Element::new_empty()
    }
}

impl Into<u32> for Page32Element {
    fn into(self) -> u32 {
        match self.inner {
            Some(val) => val.get(),
            None => 0
        }
    }
}

#[derive(Clone, Copy)]
union TableData {
    inner: [Page32Element; 1024],
    as_page: Page
}
const_assert!(core::mem::size_of::<TableData>() == 0x1000);
const_assert!(core::mem::align_of::<TableData>() == 0x1000);

const impl Default for TableData {
    fn default() -> Self {
        TableData { inner: [Default::default(); 1024] }
    }
}

impl !Unpin for TableData {}

impl TableData {
    unsafe fn set_item<A: Allocator + Clone>(self: &mut Pin<Box<Self, A>>, addr: usize, item: Page32Element) {
        unsafe {
            Pin::get_unchecked_mut(Pin::as_mut(self)).inner[addr as usize] = item;
        }
    }

    fn get_item<A: Allocator + Clone>(self: &Pin<Box<Self, A>>, addr: usize) -> Page32Element {
        return unsafe { (*self).inner }[addr];
    }

    fn get_ptr(&self) -> *const Page {
        return &raw const self.as_page;
    }

}

impl Debug for TableData {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut list = f.debug_list();
        for i in 0..1024 {
            let val: u32 = Page32Element::into(unsafe { self.inner }[i]);
            list.entry(&val);
        }
        list.finish()
    }
}

#[derive(Clone, Debug)]
struct PageTable<A: Allocator + Clone> {
    raw: Pin<Box<TableData, A>>,
    a: A
}


impl<A: Allocator + Clone> PageTable<A> {

    fn new_in(a: A) -> Self {
        let ptr = unsafe { Pin::new_unchecked(Box::new_in(TableData::default(), a.clone())) };
        Self { raw: ptr, a }
    }

    fn insert_page(&mut self, addr: *mut Page, value: *mut Page, page_type: PageType, perms: Permission) -> Result<(), PageError> {
        let write = match page_type {
            PageType::Write => true,
            PageType::Read => false,
            PageType::Execute => false
        };
        let allow_user = match perms {
            Permission::Supervisor => false,
            Permission::User => true
        };
        let entry = Page32Element::new_exists(value, false, false, false, false, false, true, allow_user, write);
        let index = (addr as usize >> 12) & 0x3FF;
        if !self.raw.get_item(index).is_empty() {
            return Err(PageError::PageExists);
        }
        unsafe { self.raw.set_item(index, entry) };
        Ok(())
    }
    
    fn get_page(&self, addr: *mut Page) -> Option<*mut Page> {
        self.raw.get_item((addr as usize >> 12) & 0x3FF).get_ptr()
    }
}

impl<A: Allocator + Clone> Drop for PageTable<A> {
    fn drop(&mut self) {
        // let orig = unsafe { Box::from_raw_in(self.raw, self.a) };

    }
}


#[derive(Clone, Debug)]
struct PageDirectory<A: Allocator + Clone> {
    // SAFETY: raw and repr must remain in sync
    raw: Pin<Box<TableData, A>>,
    repr: [Option<Rc<PageTable<A>, A>>; 1024],
    a: A
}

// impl Default for PageDirectory {
//     fn default() -> Self {
//         let raw = TableData::default();
//         let repr = [const { None }; 1024];
//         Self { raw: Box::pin(raw), repr }
//     }
// }

impl<A: Allocator + Clone> PageDirectory<A> {
    fn new_in(a: A) -> Self {
        let raw = unsafe { Pin::new_unchecked(Box::new_in(TableData::default(), a.clone())) };
        Self {
            raw: raw,
            repr: [const { None }; 1024],
            a: a
        }   
        
    }

    fn get_page_table_mut(&mut self, addr: *mut Page) -> &mut PageTable<A> {
        let addr = ((addr as u32) >> 22) as usize;

        let val  = self.repr.get_mut(addr)
            .unwrap(); // Unwrap cannot fail as max value of u10 is 1023
        let val = val.get_or_insert_with(|| Rc::new_in(PageTable::new_in(self.a.clone()), self.a.clone()));
        let val =         Rc::make_mut(val);
        unsafe {
            let item = Page32Element::new_exists(val.raw.get_ptr(), false, false, false, false, false, true, true, true);
            self.raw.set_item(addr, item);
        }
        val
    }
    
    // None if there is no entry
    fn get_page_table(&self, addr: *mut Page) -> Option<&PageTable<A>> {
        let addr = ((addr as u32) >> 22) as usize;
        let Some(ref value) = self.repr[addr] else {
            return None;
        };
        return Some(&*value);
    }
}

#[derive(Clone, Debug)]
pub struct PageMap32<A: Allocator + Clone> {
    inner: RefCell<PageDirectory<A>>,
    // Once there are more allocation modes this'll need to be more complicated
    allocptr: Cell<Option<*mut Page>>,
    a: A
}

impl<PA: Allocator + Clone> PageMap<PA> for PageMap32<PA> {
    type InsertError = PageError;

    fn new(a: PA) -> Self {
        Self { 
            inner: RefCell::new(PageDirectory::new_in(a.clone())),
            allocptr: Cell::new(None),
            a
        }
    }

    fn insert_phys(&self, addr: *mut Page, value: *mut Page, page_type: PageType, perms: Permission) -> Result<(), PageError> {
        let mut binding = self.inner.borrow_mut();
        let pt = binding.get_page_table_mut(addr);
        pt.insert_page(addr, value, page_type, perms)?;
        Ok(())
    }
    
//     fn set_phys_many(&mut self, addr: *mut Page, value: Range<*mut Page>, page_type: PageType, perms: Permission) {
//         // TODO: splitting value up into chunks of 2^22 and not calling get_page_table_mut() every time would be faster

//     }
    fn get_phys(&self, addr: *mut u8) -> Option<*mut u8> {
        let binding = self.inner.borrow();
        let page = addr.mask(!0xFFF) as *mut Page;
        let offset = addr as usize & 0xFFF;
        let Some(pt) = binding.get_page_table(page) else {
            return None;
        };
        pt.get_page(page)
            .map(|ptr| unsafe { (ptr as *mut u8).add(offset) } )
   }

   fn remove_phys(&self, addr: *mut Page) -> Result<(), Self::InsertError> {
        let binding = self.inner.borrow();
        let table = binding.get_page_table_mut(addr);
   }

   fn allocate_and_page<'a, A: Allocator + 'a>(&'a self, a: A, page_type: PageType, perms: Permission, strat: AllocationStrategy) -> PageAllocator<'a, PA, A> {
       PageAllocator { pagemap: self, local: a, strat, page_type, perms }
   }

   unsafe fn build<'a>(&'a mut self, _: &mut PageToken) -> &'a mut PageToken {
    let ptr = &raw const *self.inner.borrow().raw;
    unsafe { 
        asm!(
            "mov cr3, {ptr}",
            "mov {cr0}, cr0", // TODO: initialization function needed so that we don't do this every time
            "or {cr0}, 0x80000001",
            "mov cr0, {cr0}",
            ptr = in(reg) ptr,
            cr0 = lateout(reg) _
        );
    }

    return Box::leak(Box::new(unsafe { PageToken::new() }));
   }
}

pub struct PageAllocator<'table, PA: Allocator + Clone, A: Allocator + 'table> {
    pagemap: &'table PageMap32<PA>,
    local: A,

    strat: AllocationStrategy,
    page_type: PageType,
    perms: Permission,
}


unsafe impl<'a, PA: Allocator + Clone, A: Allocator + 'a> Allocator for PageAllocator<'a, PA, A> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let result = self.local.allocate(layout);
        if let Ok(val) = result {
            assert!(self.strat == AllocationStrategy::Kernel);
            let dst = self.pagemap.allocptr.take().unwrap();
            let len = val.len().div_ceil(0x1000);
            let src = val.as_ptr().mask(!0xFFF) as *mut Page;
            assert!(src.is_aligned());
            self.pagemap.insert_many(dst, src, len, self.page_type, self.perms).map_err(|_| AllocError)?;
            self.pagemap.allocptr.set(Some(unsafe { dst.add(len) }));
        }
        result
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.local.deallocate(ptr, layout) };
    }
}

// impl<'a, A: Allocator> PageAllocator<'a, A> {

// }

// impl<'a, A: Allocator> InstantiateAllocator for PageAllocator<'a, A> {
//     fn create<'b>(&'b self, page_type: PageType, perms: Permission) -> impl Allocator + 'b {
        
//     }
// }

// unsafe impl<A: Allocator> Allocator for PageAllocator<'_, A> {
    // fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
    //     let result = self.local.allocate(layout);
    //     if let Ok(val) = result {
    //         let len = val.len().div_ceil(0x1000);
    //         let ptr = val.as_ptr().mask(!0xFFF) as *mut Page;
    //         assert!(ptr.is_aligned());
    //         self.pagemap.set_phys_many(self.curr, ptr, len, page_type, perms);
    //     }
    //     result
    // }

//     unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {

//     }
// }
