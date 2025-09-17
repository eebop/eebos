use crate::screen::Screen;

use core::{mem, slice::from_raw_parts_mut};
use core::ptr::NonNull;
use core::arch::asm;
use core::fmt::Write;
use ::alloc::collections::BTreeMap;
use ::alloc::boxed::Box;
use core::*;
use ::alloc::vec;
use core::cell::{Ref, RefCell};
use zerocopy::{self, FromBytes, IntoBytes, KnownLayout};

#[repr(C)]
#[derive(Debug)]
pub struct SysCallData {
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

pub const SCREEN: RefCell<Screen> = RefCell::new(Screen { line: 0, row: 0 });
pub const INTERRUPTS: RefCell<BTreeMap<u8, fn(&mut SysCallData)>> = RefCell::new(BTreeMap::new()); 

impl SysCallData {
	// Interprets the syscall abi to receive a element of T
	// OS side api
	fn receive_abi<T: IntoBytes + FromBytes + KnownLayout + ?Sized>(&self) -> Option<&'static mut T> {
		let data = unsafe { 
			core::slice::from_raw_parts_mut(self.ecx as *mut u8, self.edx as usize)
		};
		match T::mut_from_bytes(data) {
			Ok(d) => Some(d),
			Err(_) => None
		}
	}

	// Configures SysCallData to read having a member of T
	// OS side api
	fn send_abi<T: IntoBytes + FromBytes + KnownLayout + ?Sized>(&mut self, mut val: Box<T>) {
		let data = val.as_mut_bytes();
		let (data, size) = (data.as_mut_ptr(), data.len());
		self.ecx = data as u32;
		self.edx = size as u32;
	}
}

pub fn submit_syscall_syscall(cmd: &mut SysCallData) {
	let data = cmd.receive_abi::<syscall_std::NewSysCall>().unwrap();
	let ptr: fn(&mut SysCallData) = unsafe { core::mem::transmute(data.ptr) };
	INTERRUPTS.borrow_mut().insert(data.channel, ptr);
	
	cmd.send_abi::<()>(Box::new(()));
}

pub fn debug_print_syscall(cmd: &mut SysCallData) {
	let data = cmd.receive_abi::<[u8]>().unwrap();
	let data = core::str::from_utf8_mut(data).unwrap();

}

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut SysCallData) {
	// Safe because we safely created the ref in assembly
    let regs = unsafe { regs.as_mut_unchecked() };
	let binding = SCREEN;
	let mut s = binding.borrow_mut();
	s.clear_screen();
	writeln!(&mut s, "data is: {}", regs.interrupt);
	writeln!(&mut s, "data: {:#x?}", regs);

	match INTERRUPTS.borrow().get(&regs.interrupt.try_into().unwrap()) {
		Some(syscall) => {
			syscall(regs);
		},
		None => {

		}
	}
}
