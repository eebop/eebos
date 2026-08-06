use core::marker::PhantomData;

use dyshared::CAllocator;
use paging::PageMap;

use crate::idt::RegState;

struct Task<A: CAllocator, PT: PageMap<A>> {
    pages: PT,
    state: RegState,
    _page_alloc: PhantomData<A>,
}

impl<A: CAllocator, PT: PageMap<A>> Task<A, PT> {
    pub fn from_fn(f: extern "C" fn() -> !, stack_top: *mut (), pages: PT) -> Self {
        assert!(stack_top.is_aligned_to(16));
        let state = RegState {
            edi: 0,
            esi: 0,
            ebp: 0,
            edx: 0,
            ecx: 0,
            ebx: 0,
            esp: stack_top as u32,
            eax: 0,
            eip: unsafe { core::mem::transmute(f) },
            cs: 8,
            eflags: 0,
            dummy: 0
        };
        Task {pages, state, _page_alloc: PhantomData }
    }
}