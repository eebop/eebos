use syscall_std::screen::Screen;

use core::{mem, slice::from_raw_parts_mut};
use core::ptr::NonNull;
use core::arch::asm;
use core::fmt::Write;
use core::*;
use core::cell::{RefMut, RefCell, Cell};

use syscall_std::SysCallData;
use syscall_std::State;

pub struct MonoThreadedCell<T> {
	pub inner: Cell<T>
}

// Only safe in single-threaded code, like this
unsafe impl<T> Sync for MonoThreadedCell<T> {}

pub static STATE: MonoThreadedCell<Option<State>> = MonoThreadedCell { inner: Cell::new(Some(
	State {
		screen: Screen {line: 0, row: 0},
		interrupts: [None; 256]
	}
))};


pub fn submit_syscall_syscall(cmd: &mut SysCallData, state: &mut State) {
	let data = cmd.receive_abi::<syscall_std::NewSysCall>();
	let ptr: fn(&mut SysCallData, &mut State) = unsafe { core::mem::transmute(data.ptr) };
	state.interrupts[data.channel as usize] = Some(ptr);


	cmd.send_abi(&());
}

pub fn debug_print_syscall(cmd: &mut SysCallData, state: &mut State) {
	let data = cmd.receive_abi::<u32>();
	writeln!(state.screen, "got the following: {}", data);
	cmd.send_abi(&());
}

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut SysCallData) {
	// Safe because we safely created the ref in assembly
    let regs = unsafe { regs.as_mut_unchecked() };

	let mut state = STATE.inner.take().expect("ERROR: interrupt occurred whist processing interrupt");


	// writeln!(&mut state.screen, "data is: {}", regs.interrupt);
	// writeln!(&mut state.screen, "data: {:#x?}", regs);
	writeln!(&mut state.screen, "interrupts are: {:#x?}", state.interrupts[regs.interrupt as usize]);


	if let Some(syscall) = state.interrupts[regs.interrupt as usize] {
		syscall(regs, &mut state);
	}

	STATE.inner.set(Some(state));
}
