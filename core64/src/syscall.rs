use shared::screen::Screen;

use core::fmt::Write;
use alloc::vec::Vec;
use core::cell::{RefCell, RefMut};
use core::arch::asm;

use shared::SysCallInternal;
use shared::State;
use shared::*;

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

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut SysCallInternal) {
    let regs = SysCallData::new(unsafe { regs.as_mut_unchecked() });

	let int = STATE.interrupts.borrow();
	let val = int[regs.interrupt as usize];
	drop(int);

	let mut s = Screen::new();
	
	writeln!(&mut s, "in interrupt... {:#x?}", *regs);

	match val {
		Syscall::Request(f) => {
			f(regs, &STATE);
			return;
		},
		Syscall::Empty => {
			panic!("Interrupt occurred but no corresponding interrupt handler\nError: {:#x?}", *regs);
		}
	}

}
