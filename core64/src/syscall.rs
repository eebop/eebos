use alloc::alloc::{Allocator, Global};
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use shared::process::Process;
use shared::screen::Screen;
use shared::std::DummyAllocator;

use core::fmt::Write;
use core::ffi::c_void;
use alloc::vec::Vec;
use core::cell::{Ref, RefCell, RefMut};
use core::arch::asm;

use shared::SysCallInternal;
use shared::*;


pub struct State {
	pub interrupts: RefCell<[Syscall; 256]>,
	pub syscalls: RefCell<BTreeMap<[u8; 8], Interface>>,
	pub curr_proc: RefCell<Option<Process<Global, DummyAllocator>>>
	// pub saves: RefCell<Vec<(SysCallInternal, usize)>>
}

// Safe as there will only be one processor
unsafe impl Sync for State {}

impl State {
	pub const fn new() -> Self {
		Self {
			interrupts: RefCell::new([Syscall::Empty; 256]),
			syscalls: RefCell::new(BTreeMap::new()),
			curr_proc: RefCell::new(None)
			// saves: RefCell::new(Vec::new())
		}
	}
}


pub static STATE: State = State::new();

// pub fn submit_syscall_syscall(cmd: &mut SysCallInternal, state: &mut State) {
// 	let data = cmd.receive_abi::<shared::NewSysCall>();
// 	state.interrupts[data.channel as usize] = Some(data.ptr);


// 	cmd.send_abi(&());
// }

// pub fn debug_print_syscall(cmd: &mut SysCallInternal, state: &mut State) {
// 	let data = cmd.receive_abi::<u32>();
// 	writeln!(state.screen, "got the following: {:x}", data);
// 	cmd.send_abi(&());
// }


// // Drop the stack, 
// fn exit(mut curr: SysCallData, state: &State) {
// 	match state.saves.borrow_mut().pop() {
// 		Some(s) => {
// 			*curr = s.0;
// 			STATE.currentProcess.replace(Some(s.1));
// 		},
// 		None => {loop {}}, // The OS has nothing to do. Spinloop until an interrupt
// 	}
// }

// pub fn submit_syscall(mut curr: SysCallData, state: &State) {
// 	let syscall: NewSysCall = curr.receive_abi::<shared::NewSysCall>();

	
// 	state.interrupts.borrow_mut()[syscall.channel as usize] = Syscall::Request(syscall.ptr);
// }

// This means that the interrupt was intended as a syscall
// We call the function by allocating a new stack, and then manually passing arguments
// We then just jump to the target
// TODO: this'll be much more complicated with paging
// Syscall 0x20
pub fn api_request(mut regs: SysCallData) {
	let data = regs.receive_abi::<([u8; 8], *mut c_void)>();
	let mut int_ref = STATE.syscalls.borrow_mut();

	let mut interface = int_ref.get_mut(&data.0).unwrap();

	if interface.in_use {
		panic!("Recursion not yet supported in syscalls. Origin: {:?}", data.0);
	} else {
		interface.in_use = true;
	}

	let stackptr = interface.sp as *mut c_void;
	
	// ABI - pass the input pointer
	let stackptr = unsafe { (stackptr as *mut SysCallInternal).sub(1) };
	unsafe { *stackptr = regs.clone()}

	let regsptr = stackptr as *mut _;
	
	let stackptr = stackptr as *mut *mut c_void;

	// round down for input arguments
	// We don't have __m128 so we can just be at a sixteenbyte
	let stackptr = unsafe { stackptr.sub(3).mask(!0xf).add(2) };

	unsafe { *stackptr = regsptr};

	let stackptr = unsafe { stackptr.sub(1) };

	unsafe {*stackptr = data.1};

	let stackptr = unsafe { stackptr.sub(1) };

	unsafe {*stackptr = core::ptr::null_mut()}; // returning from request is invalid

	regs.esp = unsafe { core::mem::transmute(stackptr) };

	regs.eip = unsafe { core::mem::transmute(interface.ip) };
	writeln!(&mut s, )
}

// Syscall 0x21
pub fn api_return(mut regs: SysCallData) {
	let data = regs.receive_abi::<([u8; 8], *mut SysCallInternal)>();
	let mut int_ref = STATE.syscalls.borrow_mut();
	let interface = int_ref.get_mut(&data.0).unwrap();

	assert!(interface.in_use);
	interface.in_use = false;
	
	*regs = unsafe {*data.1};
}

// Syscall 0x22
pub fn submit_interface(mut regs: SysCallData) {
	let data = regs.receive_abi::<([u8; 8], Interface)>();
	let mut int_ref = STATE.syscalls.borrow_mut();
	int_ref.insert(data.0, data.1);
}

// we are operating on the kernel stack
// so we can't do syscalls
// be careful!``
#[unsafe(no_mangle)]
pub extern "C" fn isr_handler(regs: *mut SysCallInternal) {
	// writeln!(Screen::new(), "in interrupt");
    let regs = SysCallData::new(unsafe { regs.as_mut_unchecked() });

	let int = STATE.interrupts.borrow();
	let val = int[regs.interrupt as usize];
	drop(int);

	let mut s = Screen::new();
	
	writeln!(&mut s, "in interrupt... {:#x?}", *regs);

	match val {
		Syscall::Request(f) => {
			f(regs);
		},
		Syscall::Empty => {
			panic!("Interrupt occurred but no corresponding interrupt handler\nError: {:#x?}", *regs);
		}
	}

}
