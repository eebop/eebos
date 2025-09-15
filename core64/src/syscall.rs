use core::mem;


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

unsafe extern "C" {
    static mut interrupts: [fn(*mut SysCallData); 256];
}

extern "C" fn submit_syscall_syscall(cmd: *mut SysCallData) {
    let cmd = unsafe { cmd.as_ref_unchecked() };
    unsafe { interrupts[cmd.ebx as usize] = core::mem::transmute(cmd) };
}

#[unsafe(no_mangle)]
extern "C" fn isr_handler {
    
}