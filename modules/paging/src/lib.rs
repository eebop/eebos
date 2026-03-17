#![no_std]
#![feature(negative_impls)]
#![feature(const_trait_impl)]
#![feature(const_default)]
#![feature(new_range_api)]
#![feature(allocator_api)]
#![allow(refining_impl_trait)]
#![feature(ptr_mask)]
#![feature(never_type)]

extern crate alloc;

use core::{error::Error, fmt::Debug};

use alloc::{alloc::Allocator, boxed::Box};
use dyshared::{CAllocator, Page};

// TODO: with multiple targets, cfgs will be necessary
pub mod page32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageType {
    Read,
    Write,
    Execute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    User,
    Supervisor
}

// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum AllocationStrategy {
//     Kernel
// }

// Represents ownership of the current in-use paging structure
pub struct PageToken(());
impl PageToken {
    // SAFETY: there may be exactly one PageToken (representing the current paging structure)
    pub const unsafe fn new() -> Self {
        Self(())
    }
}

/// Translates linear addresses to physical ones
pub trait TransToPhys {
    type MapError: Error;

    // None if the linear address has no physical mapping
    fn trans_page(&self, addr: *mut Page) -> Result<*mut Page, Self::MapError>;

    fn trans_addr(&self, addr: *mut u8) -> Result<*mut u8, Self::MapError> {
        let low_bits = (addr as usize) & 0x3FF;
        let high_bits = addr.mask(!0x3FF) as *mut Page;
        self.trans_page(high_bits).map(|addr| unsafe { addr.add(low_bits) } as *mut u8)
    }
}

///
pub trait MappedPageAllocator {
    type A: Allocator;
    type T: TransToPhys;

    fn allocate_page(&mut self, loc: *mut Page, page_type: PageType) -> Result<Box<Page, Self::A>, impl Error>;
    fn allocate_many(&mut self, loc: *mut Page, page_type: PageType, size: usize) -> Result<Box<[Page], Self::A>, impl Error>;
}

// TODO: at some point we'll want to be able to drop pages
pub trait PageMap<Alloc: CAllocator> : Clone + TransToPhys {
    fn new(a: Alloc) -> Self;
    /// addr is the linear address key, value is the physical page to map to
    /// Will not overwrite existing page
    fn insert_phys(&mut self, addr: *mut Page, value: *mut Page, page_type: PageType, perms: Permission) -> Result<(), Self::MapError>;

    fn remove_phys(&mut self, addr: *mut Page) -> Result<(), Self::MapError>;

    // /// Translate a linear to physical address
    // fn get_phys(&self, addr: *mut u8) -> Option<*mut u8>;

    /// When data is allocated with the returned allocator, it is also put into this pageMap
    /// Data shouldn't be dropped from the returned allocator, instead the PageMap should  be dropped
    fn allocate_and_page<'a, A: CAllocator + 'a, T: TransToPhys>(&'a mut self, a: A, perms: Permission, curr_map: &'a T) -> impl MappedPageAllocator<T=T, A=A> + 'a;


    fn insert_many(&mut self, addr: *mut Page, value: *mut Page, num: usize, page_type: PageType, perms: Permission) -> Result<(), Self::MapError> {
        for ind in 0..num {
            self.insert_phys(unsafe { addr.add(ind) }, unsafe { value.add(ind) }, page_type, perms)?;
        }
        Ok(())
    }
    unsafe fn build(&mut self, token: PageToken) -> PageToken;
}

pub struct IdentityMap;
impl TransToPhys for IdentityMap {
    type MapError = !;

    fn trans_page(&self, addr: *mut Page) -> Result<*mut Page, Self::MapError> {
        Ok(addr)
    }
}