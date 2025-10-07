#![no_std]
#![no_main]

use core::cell::SyncUnsafeCell;
use core::panic::PanicInfo;

use core::*;

use core::fmt::Write;

use core::alloc::GlobalAlloc;

use shared::process::Process;
use shared::State;
use shared::SysCallData;
use shared::SysCallInternal;
use shared::make_syscall;

#[panic_handler]
fn panic<'a, 'b>(p: &'a PanicInfo<'b>) -> ! {
    let mut s = shared::screen::Screen {line: 0, row: 0};
    writeln!(&mut s, "{}", p);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn __libc_start_main() {
    main();
}

struct EmptyAllocator;

unsafe impl GlobalAlloc for EmptyAllocator {
    unsafe fn alloc(&self, layout: alloc::Layout) -> *mut u8 {
        panic!("Alloc not supported here!");
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: alloc::Layout) {
        panic!("Alloc not supported here!");
    }
}

#[global_allocator]
static EMPTY_ALLOCATOR: EmptyAllocator = EmptyAllocator;

static ret_state: SyncUnsafeCell<Option<SysCallInternal>> = SyncUnsafeCell::new(None);

fn enter(mut curr: SysCallData, state: &State) {
    unsafe { ret_state.get().as_mut_unchecked() = Some(*curr) };
    let val = curr.receive_abi::<usize>();
}

fn do_init_mod(name: String) {
    // First, load the mod into memory
    make_syscall::<String, Process, 0xfe>(name);
    
}

#[unsafe(no_mangle)]
pub extern "C" fn main() {
    loop {}
}
