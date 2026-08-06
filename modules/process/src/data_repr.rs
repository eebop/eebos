use core::{cmp::Ordering, fmt::Debug, ops::{Div, Index, Range, RangeInclusive}, panic};

use alloc::{alloc::Allocator, borrow::Cow, boxed::Box, collections::btree_set::BTreeSet, vec::Vec};
use dyshared::{CAllocator, Page, PageAligned, screen::Screen};
use rangemap::{RangeInclusiveMap, RangeMap};
use core::fmt::Write;

use crate::elf::Relocation;

/// Comparing overlapping DataLocs is an error and can't happen
#[derive(Clone, Copy)]
pub struct DataLoc<'a> {
    start: *mut u8,
    filesz: usize, // in bytes
    memsz: usize, // in bytes
    data: &'a [u8]
}

impl<'a> DataLoc<'a> {
    pub fn new(data: &'a [u8], start: *mut u8, filesz: usize, memsz: usize) -> Self {
        DataLoc { start, filesz, memsz, data }
    }

    fn overlaps(&self, other: &DataLoc<'a>) -> bool {
        let (min, max) = match self.start.cmp(&other.start) {
            Ordering::Less => (self, other),
            Ordering::Greater => (other, self),
            Ordering::Equal => {
                return true
            }
        };
        (unsafe { min.start.add(min.memsz) }) > max.start
    }
}

impl<'a> Debug for DataLoc<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DataLoc")
            .field("start", &self.start)
            .field("filesz", &self.filesz)
            .field("memsz", &self.memsz)
            .finish()
    }
}

impl<'a> PartialEq for DataLoc<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
    }
}

impl<'a> Eq for DataLoc<'a> {}

impl<'a> PartialOrd for DataLoc<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for DataLoc<'a> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}

/// Borrow of data representing a object
#[derive(Debug)]
pub struct SharedDataRepr<'a, A: Allocator + Clone> {
    inner: BTreeSet<DataLoc<'a>, A>,
    a: A
}

impl<'a, A: Allocator + Clone> SharedDataRepr<'a, A> {
    pub fn new_in(a: A) -> Self {
        Self {
            inner: BTreeSet::new_in(a.clone()),
            a
        }
    }

    pub fn insert(&mut self, new: DataLoc<'a>) {
        for item in &self.inner {
            // TODO: we could binary search to find a overlap but BTreeMap doesn't have an api that support that afaik
            assert!(!item.overlaps(&new))
        }
        self.inner.insert(new);
    }

    // // Given a slice into 
    // pub fn get_slice(&self, out: &mut [u32], index: *mut [u32]) {
    //     // I don't know if a init_fn array or smth could go between two headers
    //     // No machine-generated elf would though
    //     // So it's probably fine

    //     writeln!(Screen, "out {:?} == index {:?}", out, index);

    //     assert!(out.len() == index.len() * 4);

    //     let mut result = None;

    //     let start_ptr = index as *mut u8;
    //     let end_ptr = unsafe { (index as *mut u32).add(index.len()) } as *mut u8;

    //     for val in &self.inner {
    //         if start_ptr >= val.start && start_ptr < unsafe { val.start.add(val.memsz) } {
    //             result = Some(val);
    //             unsafe {
    //                 assert!(end_ptr > val.start && end_ptr <= val.start.add(val.memsz), "Pointer in elf doesn't point to a contiguous section");
    //             }
    //         }
    //     }

    //     let result: &DataLoc<'_> = result.expect("Pointer in elf didn't point to data");

    //     let mut dummy;
    //     let src = if index.len() <= result.filesz as usize {
    //         &result.data[unsafe { start_ptr.offset_from(result.start) } as usize..][..index.len() * 4]
    //     } else {
    //         dummy = result.data[unsafe { start_ptr.offset_from(result.start) } as usize..][..index.len() * 4].to_vec_in(self.a.clone());
    //         dummy.resize(index.len() * size_of::<u8>(), 0);
    //         &dummy
    //     };
    //     out.copy_from_slice(src);
    // }

    fn start(&self) -> Option<&DataLoc<'_>> {
        return self.inner.first()
    }

    fn end(&self) -> Option<&DataLoc<'_>> {
        return self.inner.last()
    }

    // Maximum length in bytes 
    pub fn len(&self) -> usize {
        let start = self.start();
        let end = self.end();
        let (Some(first), Some(last)) = (start, end) else { 
            let (None, None) = (start, end) else {
                unreachable!("Corrupted BTreeSet (min elem but no max elem or visa versa)");
            };
            return 0; // No elements -> 0 size
        };
        let start = first.start as *mut u8;
        let end = unsafe { (last.start as *mut u8) .add(last.memsz) };
        unsafe { end.offset_from(start) as usize }
    }

    pub fn get_baseaddr(&self) -> *mut u8 {
        self.start().map(|loc| loc.start).unwrap_or(core::ptr::null_mut())
    }

    pub fn write(&self, dest: &mut [Page], rel: Relocation) {
        writeln!(Screen, "dest has {:x?}, size is {:x?}", dest.len(), self.len());
        for loc in self.inner.iter() {
            let this = &mut dest.as_contiguous()[rel.relocate_slice(loc.start as u32) as usize..];
            this[..loc.filesz].copy_from_slice(loc.data);
            this[loc.filesz..loc.memsz].fill(0);
        }
    }
}