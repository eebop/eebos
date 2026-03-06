#![no_std]
#![feature(negative_impls)]
#![feature(const_trait_impl)]
#![feature(const_default)]
#![feature(new_range_api)]
#![feature(allocator_api)]
#![allow(refining_impl_trait)]
#![feature(ptr_mask)]

extern crate alloc;

use core::fmt::Debug;

use alloc::alloc::Allocator;
use dyshared::Page;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationStrategy {
    Kernel
}

// Represents ownership of the current in-use paging structure
pub struct PageToken(());
impl PageToken {
    // SAFETY: there may be exactly one PageToken (representing the current paging structure)
    pub const unsafe fn new() -> Self {
        Self(())
    }
}

// TODO: at some point we'll want to be able to drop pages
pub trait PageMap<Alloc: Allocator + Clone> : Clone {
    type InsertError : Debug;

    fn new(a: Alloc) -> Self;
    /// addr is the linear address key, value is the physical page to map to
    /// Will not overwrite existing page
    fn insert_phys(&self, addr: *mut Page, value: *mut Page, page_type: PageType, perms: Permission) -> Result<(), Self::InsertError>;

    fn remove_phys(&self, addr: *mut Page) -> Result<(), Self::InsertError>;

    /// Translate a linear to physical address
    fn get_phys(&self, addr: *mut u8) -> Option<*mut u8>;

    /// When data is allocated with the returned allocator, it is also put into this pageMap
    /// Data shouldn't be dropped from the returned allocator, instead the PageMap should  be dropped
    fn allocate_and_page<'a, A: Allocator + 'a>(&'a self, a: A, page_type: PageType, perms: Permission, alloc: AllocationStrategy) -> impl Allocator + 'a;


    fn insert_many(&self, addr: *mut Page, value: *mut Page, num: usize, page_type: PageType, perms: Permission) -> Result<(), Self::InsertError>{
        for ind in 0..num {
            self.insert_phys(unsafe { addr.add(ind) }, unsafe { value.add(ind) }, page_type, perms)?;
        }
        Ok(())
    }
    unsafe fn build<'a>(&'a mut self, token: &mut PageToken) -> &'a mut PageToken;
}