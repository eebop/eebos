use core::arch::{asm, naked_asm};

use alloc::boxed::Box;
use dyshared::{CAllocator, screen::Screen};
use core::fmt::Write;

#[derive(Clone, Copy, Debug)]
struct IDTEntry {
    offset: u32,
    segment: u16,
    gate: u8,
    privilege: u8   
}

impl IDTEntry {
    fn encode(self: Self) -> [u8; 8] {
        let mut target: [u8; 8] = [0; 8];
        target[0] = ((self.offset >> 0x00) & 0xFF) as u8;
        target[1] = ((self.offset >> 0x08) & 0xFF) as u8;
        target[6] = ((self.offset >> 0x10) & 0xFF) as u8;
        target[7] = ((self.offset >> 0x18) & 0xFF) as u8;

        target[2] = ((self.segment >> 0x00) & 0xFF) as u8;
        target[3] = ((self.segment >> 0x08) & 0xFF) as u8;

        target[5] |= self.gate & 0xF;
        target[5] |= (self.privilege & 0x3) << 4;
        target[5] |= 1 << 7;

        return target;
    }

    fn encode_empty(self: Self) -> [u8; 8] {
        [0; 8]
    }
}



#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RegState {
	pub eip: u32,
	pub cs: u32, // actually u16 (u32 for alignment reasons)
	pub eflags: u32,
	pub edi: u32,
	pub esi: u32,
	pub ebp: u32,
    pub dummy: u32,
	pub ebx: u32,
	pub edx: u32,
	pub ecx: u32,
	pub eax: u32,
	pub esp: u32,
}


fn rejoin_task(state: RegState) -> ! {
    todo!()
}

extern "C" fn test_irq_recv(data: *mut RegState) -> ! {
    writeln!(Screen, "data is: {:?}", data);
    rejoin_task(unsafe { core::ptr::read(data) });
}

static mut stack_top: *mut () = core::ptr::null_mut();

// TODO: each interrupt should have a callback instead of just this hardcoded function
#[unsafe(naked)]
extern "C" fn test_irq_handler() {
    naked_asm!(
        "mov [{stack_top}], esp",
        "mov esp, {stack_top} + 4",
        "pushad",
        "mov eax, [{stack_top}]",
        "push DWORD PTR [eax + 0x8]", // eip
        "push DWORD PTR [eax + 0x4]", // cs
        "push DWORD PTR [eax + 0x0]", // eflags
        
        "lea eax, [esp]",

        "sub esp, 0x08",
        "and esp, 0xFFFFFFF0",
        "mov [esp + 0x4], eax",
        "jmp {fnptr}",
        stack_top = sym stack_top,
        fnptr = sym test_irq_recv
    )
}

pub(crate) fn init_interrupts<A: CAllocator + 'static>(a: A) {
    let data = [[0u8; 8]; 256];

    // Must be static
    let data: &'static mut [[u8; 8]; 256] = Box::leak(Box::new_in(data, a));

    let entry = IDTEntry {
        offset: unsafe { core::mem::transmute(test_irq_handler as extern "C" fn())},
        segment: 0x8,
        gate: 0xE, // interrupt, disables interrupts until IRET
        privilege: 0
    };

    for i in 0..256 {
        data[i] = entry.encode();
    }
    
    #[repr(C, packed)]
    struct IDT {
        size: u16,
        ptr: u32
    }

    let result = IDT {
        size: 256 * 8 - 1,
        ptr: unsafe {core::mem::transmute(data)}
    };

    unsafe { asm!(
        "lidt [{ptr}]",
        "sti",
        ptr = in(reg) &raw const result

    ) }
}