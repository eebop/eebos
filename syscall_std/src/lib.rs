#![no_std]

use core::*;
use core::arch::asm;
use zerocopy;
use zerocopy::{IntoBytes, FromBytes, KnownLayout};

// Makes a syscall and then interprets the return value
// User side api
pub fn make_syscall<T: IntoBytes + FromBytes + KnownLayout, U: IntoBytes + FromBytes + KnownLayout, const CHANNEL: u8>(mut data: T) -> Option<&'static mut U> {
	let data = data.as_mut_bytes();
	let (mut data, mut size) = (data.as_mut_ptr(), data.len() as u32);
	unsafe { asm! (
		"mov eax, esp",
		"int {0}",
		"mov esp, eax",
		const CHANNEL,
		inlateout("ecx") data,
		inlateout("edx") size
	) };
	let data: &mut [u8] = unsafe { slice::from_raw_parts_mut(data, size as usize) };

	match U::mut_from_bytes(data) {
		Ok(d) => Some(d),
		Err(_) => None
	}
}

#[derive(IntoBytes, FromBytes, KnownLayout)]
#[repr(packed)]
pub struct NewSysCall {
	pub channel: u8,
	pub ptr: u32
}
