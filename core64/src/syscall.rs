use crate::screen::Screen;

use core::{mem, slice::from_raw_parts_mut};
use core::ptr::NonNull;
use core::arch::asm;
use core::fmt::Write;
use core::*;
use alloc::collections::BTreeMap;
use ::alloc::vec;
use core::cell::{Ref, RefCell};
use zerocopy::{self, FromBytes, IntoBytes, KnownLayout};

#[repr(C)]
#[derive(Debug)]
struct SysCallData {
	interrupt: u32, // actually u8 (u32 for alignment reasons)
	edi: u32,
	esi: u32,
	ebp: u32,
	edx: u32,
	ecx: u32,
	ebx: u32,
	eax: u32,
	esp: u32,
	eip: u32,
	cs: u32, // actually u16 (u32 for alignment reasons)
	eflags: u32,
}

const SCREEN: RefCell<Screen> = RefCell::new(Screen { line: 0, row: 0 });
const INTERRUPTS: RefCell<BTreeMap<u8, fn(&SysCallData)>> = RefCell::new(BTreeMap::new()); 

// Makes a syscall and then interprets the return value
// User side api
fn make_syscall<T: IntoBytes + FromBytes + KnownLayout, U: IntoBytes + FromBytes + KnownLayout, const channel: u8>(mut data: T) -> Option<&'static mut U> {
	let data = data.as_mut_bytes();
	let (mut data, mut size) = (data.as_mut_ptr(), data.len() as u32);
	unsafe { asm! (
		"mov eax, esp",
		"int {0}",
		"mov esp, eax",
		const channel,
		inlateout("ecx") data,
		inlateout("edx") size
	) };
	let data: &mut [u8] = unsafe { slice::from_raw_parts_mut(data, size as usize) };

	match U::mut_from_bytes(data) {
		Ok(d) => Some(d),
		Err(_) => None
	}
}

// Interprets the syscall abi to receive a element of T
// OS side api
fn receive_abi<T: IntoBytes + FromBytes + KnownLayout>(data: &SysCallData) -> Option<&'static mut T> {
	let data = unsafe { 
		core::slice::from_raw_parts_mut(data.ecx as *mut u8, data.edx as usize)
	};
	match T::mut_from_bytes(data) {
		Ok(d) => Some(d),
		Err(_) => None
	}
}

// Configures SysCallData to read having a member of T
// OS side api
fn send_abi<T: IntoBytes + FromBytes + KnownLayout>(mut val: T, regs: &mut SysCallData) {
	let data = val.as_mut_bytes();
	let (data, size) = (data.as_mut_ptr(), data.len());
	regs.ecx = data as u32;
	regs.edx = size as u32;
}

// fn submit_syscall_syscall(cmd: &SysCallData) {
//     INTERRUPTS.borrow_mut().insert(key, value)
// }

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut SysCallData) {
	// Safe because we safely created the ref in assembly
    let regs = unsafe { regs.as_mut_unchecked() };
	let binding = SCREEN;
	let mut s = binding.borrow_mut();
	s.clear_screen();
	writeln!(&mut s, "data is: {}", regs.interrupt);
	writeln!(&mut s, "data: {:#x?}", regs);

	loop {};

	match INTERRUPTS.borrow().get(&regs.interrupt.try_into().unwrap()) {
		Some(syscall) => {
			syscall(regs);
		},
		None => {

		}
	}
}
