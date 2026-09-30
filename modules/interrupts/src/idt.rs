use core::{arch::{asm, naked_asm}, ptr};

use alloc::boxed::Box;
use dyshared::{CAllocator, bochsdbg, screen::Screen};
use pic::send_eoi;
use core::fmt::Write;

use crate::interrupts::UNIMPLEMENTED;

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
	pub edi: u32,
	pub esi: u32,
	pub ebp: u32,
    pub _dummy: u32, // pusha pushes esp for some reason
	pub esp: u32,
	pub edx: u32,
	pub ecx: u32,
	pub eax: u32,
	pub eflags: u32,
	pub cs: u32, // actually u16 (u32 for alignment reasons)
	pub eip: u32,
    pub ebx: u32,
}

impl RegState {
    pub fn from_fn(f: extern "C" fn(), stack_top: *mut u32) -> Self {
        assert!(stack_top.is_aligned_to(16));
        let stack_top = unsafe { stack_top.sub(4) }; // 16 bytes are saved for ret
        RegState {
            edi: 0,
            esi: 0,
            ebp: 0,
            edx: 0,
            ecx: 0,
            ebx: 0,
            esp: stack_top as u32,
            eax: 0,
            eip: unsafe { core::mem::transmute(f) },
            cs: 8,
            eflags: 0x202, // TDOO: better way to get this? (I/O + Int flags)
            _dummy: 0
        }
    }
}


fn test_report_interrupt(data: RegState) -> ! {
    writeln!(Screen, "interrupt: {data:?}");
    bochsdbg();
    send_eoi(1);
    rejoin_task(data)
}

pub fn rejoin_task(state: RegState) -> ! {
    // bochsdbg();
    unsafe { asm!(
        "mov esp, eax",
        "popad",
        "pop [ebx+12]",
        "pop [ebx+8]",
        "pop [ebx+4]",
        "xchg ebx, esp",
        "mov ebx, [ebx]",
        "add esp, 4",
        "iretd",
        in("eax") &state,
        options(noreturn)
    ) }
}

extern "C" fn irq_stage2(data: *mut RegState) -> ! {
    let data = unsafe { ptr::read(data) }; // Necessary because may be overwritten
    // schedule(data)
    test_report_interrupt(data)
}

pub(crate) fn init_interrupts<A: CAllocator + 'static>(a: A) {
    let data = [UNIMPLEMENTED; 256];
    lidt(data);
}


fn lidt(fns: [unsafe extern "custom" fn(); 256]) {
    let data = [[0u8; 8]; 256];

    writeln!(Screen, "here in init_interrupts");

    // Must be static
    let data: &'static mut [[u8; 8]; 256] = Box::leak(Box::new(data));

    for i in 0..256 {
        let ptr = fns[i];
        
        let entry = IDTEntry {
            offset: unsafe { core::mem::transmute(ptr)},
            segment: 0x8,
            gate: 0xE, // interrupt, disables interrupts until IRET
            privilege: 0
        };


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
        ptr = in(reg) &raw const result

    ) }
}