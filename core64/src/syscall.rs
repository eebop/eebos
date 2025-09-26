use shared::screen::Screen;

use core::fmt::Write;
use alloc::vec::Vec;
use core::cell::{RefCell, RefMut};

use shared::SysCallInternal;
use shared::State;
use shared::*;

pub static STATE: State = State::new();

unsafe extern "C" {
    static mut stored_sp: u32;
    static mut stack_top: u8;
}

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

fn next_task(curr: &mut SysCallInternal, state: &State) {
	*curr = match state.saves.borrow_mut().pop() {
		Some(s) => s,
		None => {loop {}}, // The OS has nothing to do. Spinloop until an interrupt
	}
}

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut SysCallInternal) {
	// WARNING
    let regs = SysCallData::new(unsafe { regs.as_mut_unchecked() });

	match STATE.interrupts.borrow()[regs.interrupt as usize].clone() {
		Syscall::Request(f) => {
			f(regs, &STATE);
			return;
		},
		Syscall::Dispatch(f) => {
			// We are diverging, so the next interrupt should restart the stack
			unsafe {
				stored_sp = (&raw mut stack_top) as u32;
			}
			STATE.saves.borrow_mut().push(*regs);
			todo!();
		},
		Syscall::Empty => {
			panic!("Interrupt occurred but no corresponding interrupt handler\nError: {:#?}", *regs);
		}
	}

}
