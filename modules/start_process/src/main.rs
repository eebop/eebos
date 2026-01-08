#![no_std]
#![no_main]

#![feature(sync_unsafe_cell)]
#![feature(ptr_as_ref_unchecked)]
#![feature(allocator_api)]

use core::cell::SyncUnsafeCell;
use core::panic::PanicInfo;

use core::*;

use core::prelude;

use core::fmt::Write;

use core::alloc::GlobalAlloc;
use core::alloc::Layout;

use alloc::alloc::Global;
use shared::process::Process;
use shared::NewSysCall;
use shared::State;
use shared::SysCallData;
use shared::SysCallInternal;
use shared::make_syscall;
use shared::screen::Screen;
extern crate alloc;
use alloc::string::String;
use shared::std::DummyAllocator;

#[panic_handler]
fn panic<'a, 'b>(p: &'a PanicInfo<'b>) -> ! {
    let mut s = shared::screen::Screen {line: 0, row: 0};
    writeln!(&mut s, "{}", p);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn __libc_start_main() -> ! {
    main();
    loop {}
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

#[global_allocator]
static EMPTY_ALLOCATOR: EmptyAllocator = EmptyAllocator;

static RET_STATE: SyncUnsafeCell<Option<SysCallInternal>> = SyncUnsafeCell::new(None);

fn enter(mut curr: SysCallData, state: &State) {
    *unsafe { RET_STATE.get().as_mut_unchecked() } = Some(*curr);
    let mut proc: Process<DummyAllocator, Global> = curr.receive_abi();
    proc.make_fncall(proc._start,Global);
}

fn exit(mut curr: SysCallData, state: &State) {
    *curr = unsafe { *RET_STATE.get().as_mut_unchecked() }.unwrap();
    unsafe { *RET_STATE.get() = None }
}

fn do_init_mod(name: String) {
    // First, load the mod into memory
    let proc = make_syscall::<String, Process<DummyAllocator, Global>, 0xfe>(name);
    // Then, fire enter(). It'll have to call exit()
    make_syscall::<Process<DummyAllocator, Global>, (), 0x40>(proc);
    
    proc
}

#[unsafe(no_mangle)]
pub extern "C" fn main() {
    let mut s = Screen { line: 0, row: 0};

    make_syscall::<_, (), 0x30>(NewSysCall {channel: 0x40, ptr: enter});
    make_syscall::<_, (), 0x30>(NewSysCall {channel: 0x41, ptr: exit});

    writeln!(&mut s, "test_here");
    loop {}

}
