#![no_std]

use core::*;
use core::arch::asm;
use core::mem::MaybeUninit;

pub mod screen;
pub mod ports;
use screen::Screen;

// Makes a syscall and then interprets the return value
// User side api
pub fn make_syscall<T: Copy, U: Copy, const CHANNEL: u8>(mut data: T) -> U {
	let mut out: MaybeUninit<U> = MaybeUninit::uninit();
	unsafe { asm! (
		"mov eax, esp",
		"int {0}",
		"mov esp, eax",
		const CHANNEL,
		in("ecx") &raw mut data,
		in("edx") &raw mut out,
	) };
	
	unsafe { out.assume_init() }
}

#[derive(Copy, Clone)]
pub struct NewSysCall {
	pub channel: u8,
	pub ptr: fn(&mut SysCallData, &mut State)
}

#[repr(C)]
#[derive(Debug)]
pub struct SysCallData {
	pub interrupt: u32, // actually u8 (u32 for alignment reasons)
	edi: u32,
	esi: u32,
	ebp: u32,
	edx: u32,
	ecx: u32,
	ebx: u32,
	esp: u32,
	eax: u32,
	eip: u32,
	cs: u32, // actually u16 (u32 for alignment reasons)
	eflags: u32,
}

pub struct State {
	pub screen: Screen,
	pub interrupts: [Option<extern "C" fn(&mut SysCallData, &mut State)>; 256],
}

impl SysCallData {
	// Interprets the syscall abi to receive a element of T
	// OS side api
	pub fn receive_abi<T: Copy>(&self) -> T {
		let data = self.ecx as *const T;
		unsafe { *data }.clone() // We must clone bc data is owned by caller
	}

	// Configures SysCallData to read having a member of T
	// OS side api
	pub fn send_abi<T: Copy>(&mut self, val: &T) {
		let ptr = self.edx as *mut T;
		unsafe { *ptr = *val };
	}
}
