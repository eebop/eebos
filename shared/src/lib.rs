#![no_std]
#![feature(sync_unsafe_cell)]
#![feature(ptr_as_ref_unchecked)]
#![feature(allocator_api)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(iter_collect_into)]
#![macro_use]

#![allow(unused)]
extern crate alloc;

use core::ffi::c_void;
use core::*;
use core::arch::asm;
use core::mem::MaybeUninit;

pub mod screen;
pub mod ports;
pub mod process;
pub mod cpuinfo;
// pub mod api;
pub mod std;
use screen::Screen;
use core::cell::{RefCell};
use alloc::{vec::Vec};
use alloc::collections::BTreeMap;
use core::ops::{Deref, DerefMut};
use core::fmt::Write;

use crate::process::Process;

// Makes a syscall and then interprets the return value
// User side api
pub fn make_syscall<T, U, const CHANNEL: u8>(mut data: T) -> U {
	// writeln!(Screen::new(), "here in make_syscall (T: {}, U: {}, CHANNEL is {}", core::any::type_name::<T>(), core::any::type_name::<U>(), CHANNEL);
	// panic!();
	let mut out: MaybeUninit<U> = MaybeUninit::uninit();
	// TODO: this will be more complicated with paging, as we can't just pass pointers
	unsafe { asm! (
		"mov eax, esp", // Rust garentees that there is no redzone
		"int {0}",
		"mov esp, eax", // We must restore esp as it is corrupted however
		const CHANNEL,
		lateout("eax") _,
		in("ecx") &raw mut data,
		in("edx") &raw mut out,
	) };
	
	unsafe { out.assume_init() }
}

// #[derive(Copy, Clone)]
// pub struct NewSysCall {
// 	pub channel: u8,
// 	pub ptr: fn(SysCallData)
// }

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysCallInternal {
	pub interrupt: u32, // actually u8 (u32 for alignment reasons)
	pub edi: u32,
	pub esi: u32,
	pub ebp: u32,
	pub edx: u32,
	pub ecx: u32,
	pub ebx: u32,
	pub esp: u32,
	pub eax: u32,
	pub eip: u32,
	pub cs: u32, // actually u16 (u32 for alignment reasons)
	pub eflags: u32,
}

impl SysCallInternal {
	// Interprets the syscall abi to receive a element of T
	// OS side api
	pub fn receive_abi<T>(&self) -> T {
		let data = unsafe { (self.ecx as *mut MaybeUninit<T>).as_mut_unchecked() };
		let out: MaybeUninit<T> = core::mem::replace(data, MaybeUninit::uninit());
		unsafe { out.assume_init() }
	}

	// Configures SysCallInternal to read having a member of T
	// OS side api
	pub fn send_abi<T>(&mut self, val: T) {
		let ptr = self.edx as *mut T;
		unsafe { *ptr = val };
	}

}

#[derive(Debug)]
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
}

#[derive(Clone, Copy, Debug)]
pub struct Interface {
	pub ip: usize,
	pub sp: usize,
	pub in_use: bool // will need to be Lock if multithreading
}

#[derive(Default, Clone, Copy, Debug)]
pub enum Syscall {
	// A request claims a mutable lock on the OS state
	// Used to modify core state
	Request(fn(SysCallData)),
	#[default]
	Empty
}

// Breakpoint using bochs
pub fn bochsdbg() {
	unsafe { core::arch::asm!("xchg bx, bx") };
}