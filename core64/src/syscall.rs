use core::mem;
use core::ptr::NonNull;
use core::arch::asm;
use alloc::collections::BTreeMap;
use core::cell::RefCell;

#[repr(C)]
struct SysCallData {
	interrupt: u32,
	edi: u32,
	esi: u32,
	ebp: u32,
	edx: u32,
	ecx: u32,
	ebx: u32,
	eax: u32,
	esp: u32,
	eip: u32,
	cs: u32, // upper 16 bits must be 0 (must be u32 for alignment reasons)
	eflags: u32,
}

const INTERRUPTS: RefCell<BTreeMap<u8, fn(&SysCallData)>> = RefCell::new(BTreeMap::new()); 

// Makes a syscall and then interprets the return value
// User side api
fn make_syscall<T, U, const reg: u8>(data: T, channel: u8) -> U {
	unsafe { asm!(
		"mov eax, esp",
		"int {0}",
		"mov esp, eax",
		in(reg) channel,
		out("eax") _,
		out("ebx") _,
		out("ecx") _,
	) };
	todo!();
}

// Interprets the syscall abi to receive a element of T
// OS side api
fn receive_abi<T>(data: &SysCallData) -> T {
	let data = unsafe { core::slice::from_raw_parts(data.ebx as *const u8, data.ecx as usize) };
	
}

// Configures SysCallData to read having a member of T
// OS side api
fn send_abi<T>(val: T, data: &mut SysCallData) {

}

fn submit_syscall_syscall(cmd: &SysCallData) {
    INTERRUPTS.borrow_mut().insert(cmd.ebx, );
}

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut SysCallData) {
	// Safe because we safely created the ref in assembly
    let regs = unsafe { regs.as_mut_unchecked() };
        match INTERRUPTS.borrow().get(regs.interrupt as u8) {
		Some(syscall) => {
			syscall(regs);
		},
		None => {

		}
	}
}
