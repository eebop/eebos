#![no_std]
#![no_main]

#![feature(sync_unsafe_cell)]
#![feature(ptr_as_ref_unchecked)]
#![feature(allocator_api)]
#![feature(never_type)]

use core::cell::SyncUnsafeCell;
use core::panic::PanicInfo;

use core::*;

use core::prelude;

use core::fmt::Write;

use core::alloc::GlobalAlloc;
use core::alloc::Layout;

use alloc::alloc::Global;
use alloc::boxed::Box;
use alloc::string::ToString;
use shared::Interface;
use shared::process::Page;
use shared::process::Process;
use shared::SysCallData;
use shared::SysCallInternal;
use shared::make_syscall;
use shared::screen::Screen;
use shared::{syscall, syscall_manual_return};


extern crate alloc;
use alloc::string::String;
use shared::std::DummyAllocator;
use shared::std::SimpleAllocator;

#[panic_handler]
fn panic<'a, 'b>(p: &'a PanicInfo<'b>) -> ! {
    let mut s: Screen = shared::screen::Screen {line: 0, row: 0};
    s.clear_screen();
    writeln!(&mut s, "panic!");
    writeln!(&mut s, "{}", p);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn __libc_start_main() -> ! {
    main();
    loop {}
}


#[global_allocator]
static EMPTY_ALLOCATOR: SimpleAllocator = SimpleAllocator;

static RET_STATE: SyncUnsafeCell<Option<SysCallInternal>> = SyncUnsafeCell::new(None);

fn enter(mut proc: Process<DummyAllocator, Global>, mut data: SysCallData) {
    // *unsafe { RET_STATE.get().as_mut_unchecked() } = Some(*data);
    // proc.make_fncall(proc._start,Global);
    writeln!(Screen::new(), "data is: {data:?}");
}

syscall_manual_return!(enter_syscall = enter, Process<DummyAllocator, Global>, (), *b"sysenter");

fn exit(mut curr: SysCallData) {
    let mut s = Screen::new();
    writeln!(&mut s, "got to exit()!!!");
    writeln!(&mut s, "test {:?}", 0);
    loop {}
    // *curr = unsafe { *RET_STATE.get().as_mut_unchecked() }.unwrap();
    // unsafe { *RET_STATE.get() = None }
}

fn do_init_mod(name: &str) -> Process<DummyAllocator, Global> {
    // First, load the mod into memory
    let proc = make_syscall::<&str, Process<DummyAllocator, DummyAllocator>, 0xfe>(name);
    let mut proc = proc.try_own_none(Global).unwrap();
    // Then, fire enter(). It'll have to call exit()
    make_syscall::<([u8; 8], *mut Process<DummyAllocator, Global>), (), 0x20>((*b"sysenter", &raw mut proc));
    
    proc
}

fn make_interface(f: extern "C" fn(*mut core::ffi::c_void, *mut SysCallInternal)) -> Interface {
    let new_stack = Page::uninit_many(16, Global);
    let ptr = Box::into_raw(new_stack);
    Interface { ip: unsafe { core::mem::transmute(f) }, sp: unsafe { core::mem::transmute((ptr as *mut u8).add(0x10000)) }, in_use: false }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() {
    // Interface::new()
    writeln!(Screen::new(), "test_here");
    make_syscall::<_, (), 0x22>((*b"sysenter", make_interface(enter_syscall)));
    // make_syscall::<_, (), 0x30>(NewSysCall {channel: 0x40, ptr: enter});
    // make_syscall::<_, (), 0x30>(NewSysCall {channel: 0x41, ptr: exit});

    let mut s: Screen = Screen::new();
    do_init_mod("pic");

    writeln!(&mut s, "result is:");

    // let x = do_init_mod("pic".to_string());
    loop {}

}
