use shared::screen::Screen;

use core::fmt::Write;
use core::*;
use core::cell::Cell;

use shared::SysCallData;
use shared::State;

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
	let data = cmd.receive_abi::<shared::NewSysCall>();
	state.interrupts[data.channel as usize] = Some(data.ptr);


	cmd.send_abi(&());
}

pub fn debug_print_syscall(cmd: &mut SysCallData, state: &mut State) {
	let data = cmd.receive_abi::<u32>();
	writeln!(state.screen, "got the following: {:x}", data);
	cmd.send_abi(&());
}


#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut SysCallData) {

	// Safe because we safely created the ref in assembly
    let regs = unsafe { regs.as_mut_unchecked() };

	let mut state = match STATE.inner.take() {
		Some(state) => state,
		None => {
			panic!("recursive fail");
			return;
		} // Just silently ignore
						 // Hopefully just the system clock
	};


	// writeln!(&mut state.screen, "data is: {}", regs.interrupt);
	// writeln!(&mut state.screen, "data: {:#x?}", regs);
	// writeln!(&mut state.screen, "interrupts are: {:#x?}", state.interrupts[regs.interrupt as usize]);


	if let Some(syscall) = state.interrupts[regs.interrupt as usize] {
		syscall(regs, &mut state);
	}

	STATE.inner.set(Some(state));
}
