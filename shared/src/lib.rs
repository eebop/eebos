#![no_std]

extern crate alloc;

use core::*;
use core::arch::asm;
use core::mem::MaybeUninit;

pub mod screen;
pub mod ports;
pub mod process;
use screen::Screen;
use core::cell::{RefCell, RefMut};
use alloc::{boxed::Box, vec::Vec};
use core::ops::{Deref, DerefMut};

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
	pub ptr: fn(&mut SysCallInternal, &mut State)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysCallInternal {
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

impl SysCallInternal {
	// Interprets the syscall abi to receive a element of T
	// OS side api
	pub fn receive_abi<T: Copy>(&self) -> T {
		let data = self.ecx as *const T;
		unsafe { *data }.clone() // We must clone bc data is owned by caller
	}
}

pub struct SysCallData<'a> {
	inner: &'a mut SysCallInternal
}

impl Deref for SysCallData<'_> {
	type Target = SysCallInternal;
	fn deref(&self) -> &Self::Target {
		self.inner
	}
}

impl DerefMut for SysCallData<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.inner		
	}
}

impl<'a> SysCallData<'a> {

	pub fn new(data: &'a mut SysCallInternal) -> Self {
		Self {inner: data}
	}

	// Configures SysCallInternal to read having a member of T
	// OS side api
	pub fn send_abi<T: Copy>(self, val: &T) {
		let ptr = self.edx as *mut T;
		unsafe { *ptr = *val };
	}
}

#[derive(Default, Clone, Copy)]
pub enum Syscall {
	// A request claims a mutable lock on the OS state
	// Used to modify core state
	Request(fn(SysCallData, &State)),
	Dispatch(extern "C" fn() -> !),
	#[default]
	Empty
}

pub struct State {
	pub screen: RefCell<Screen>,
	pub interrupts: RefCell<[Syscall; 256]>,
	pub saves: RefCell<Vec<SysCallInternal>>
}

// Safe as there will only be one processor
unsafe impl Sync for State {}

impl State {
	pub const fn new() -> Self {
		Self {
			screen: RefCell::new(Screen {line: 0, row: 0}),
			interrupts: RefCell::new([Syscall::Empty; 256]),
			saves: RefCell::new(Vec::new())
		}
	}
}
