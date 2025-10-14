#![no_std]
#![no_main]

use core::cell::SyncUnsafeCell;
use core::panic::PanicInfo;

use core::*;

use core::fmt::Write;

use core::alloc::GlobalAlloc;

use shared::process::Process;
use shared::NewSysCall;
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
    *unsafe { ret_state.get().as_mut_unchecked() } = Some(*curr);
    let mut proc: Process = curr.receive_abi();
    proc.make_fncall(proc._start);
}

fn exit(mut curr: SysCallData, state: &State) {
    *curr = unsafe { *ret_state.get().as_mut_unchecked() }.unwrap();
    unsafe { *ret_state.get() = None }
}

fn do_init_mod(name: String) -> Process {
    // First, load the mod into memory
    let proc = make_syscall::<String, Process, 0xfe>(name);
    // Then, fire enter(). It'll have to call exit()
    make_syscall::<Process, (), 0x40>(proc);
    
    proc    
}

#[unsafe(no_mangle)]
pub extern "C" fn main() {
    make_syscall::<_, (), 0x30>(NewSysCall {channel: 0x40, ptr: enter});
    make_syscall::<_, (), 0x30>(NewSysCall {channel: 0x41, ptr: exit});
    loop {}
}
