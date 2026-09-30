use core::{arch::{asm, naked_asm}, ptr, sync::atomic::Ordering::{Acquire, Release}};

use crate::{idt::{RegState, rejoin_task}, runtime, single_threaded::SingleThreadToken};

macro_rules! interrupt {
    ($f:expr) => {{
        #[unsafe(naked)]
        unsafe extern "custom" fn i() {
            naked_asm!(
                "push %ebx",
                "call 1f",
                "1:",
                "popl %ebx",
                "addl $_GLOBAL_OFFSET_TABLE_+[.-1b], %ebx",
                "mov stack_top@GOT(%ebx), %ebx",
                "xchg %ebx, %esp",
                "push (%ebx)",
                "push 4(%ebx)",
                "push 8(%ebx)",
                "push 12(%ebx)",
                "pusha",
                "mov %esp, %eax",
                "sub $0x8, %esp",
                "and $0xFFFFFFF0, %esp",
                "add $0x8, %esp",
                "push %eax",
                "call {stage2}",
                stage2 = sym stage2,
                options(att_syntax)
            );
        }
        fn stage2(data: *mut RegState) -> ! {
            assert!(data.is_aligned());
            let data = unsafe { ptr::read(data) };
            let mut token = unsafe { SingleThreadToken::new() } ;
            let ctrl = $f(data, token.reborrow());
            match ctrl {
                CtrlFlow::Resume => {
                    rejoin_task(data)
                },
                CtrlFlow::Break(code) => {
                    let (Some(schedule), token) = runtime::RT.get(&mut token) else {
                        panic!("missing schedule")
                    };
                    schedule.next(token, code)
                }
            }
        }
        i
    }}
}

// An interrupt takes a RegState
// It statically knows what interrupt it came from
// If it resumes, nothing happens
// If it breaks, the task is annotated
enum CtrlFlow {
    Resume,
    Break(Interrupt)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    Clock,
    SysInt(SysInt)
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SysInt {
    Finish = 0,
    AsyncSleep = 1,

}

fn unimplemented_int(regs: RegState, token: SingleThreadToken) -> CtrlFlow {
    panic!("Interrupt is unimplemented: {regs:?}");
}

pub const UNIMPLEMENTED: unsafe extern "custom" fn() = interrupt!(unimplemented_int);

fn sys_int(regs: RegState, token: SingleThreadToken) -> CtrlFlow {
    let val = match regs.eax {
        0 => SysInt::Finish,
        1 => SysInt::AsyncSleep,
        _ => unreachable!()
    };
    CtrlFlow::Break(Interrupt::SysInt(val))
    
}

pub const TEST: unsafe extern "custom" fn() = interrupt!(sys_int);


pub fn raise_int(val: SysInt) {
    let val = val as u32;
    core::sync::atomic::fence(Release);
    unsafe {
        asm!(
            "int 0x79",
            in("eax") val
        )
    }
    core::sync::atomic::fence(Acquire);
}


fn stage2(data: *mut RegState) -> ! {
    assert!(data.is_aligned());
    let data = unsafe { ptr::read(data) };
    let mut token = unsafe { SingleThreadToken::new() } ;
    let ctrl = sys_int(data, token.reborrow());
    match ctrl {
        CtrlFlow::Resume => {
            rejoin_task(data)
        },
        CtrlFlow::Break(code) => {
            let Some(schedule) = runtime::RT.get(&mut token) else {
                panic!("missing schedule")
            };
            schedule.next(token, code)
        }
    }
}


// #[unsafe(naked)]
// extern "C" fn test_irq_handler() {
//     // LLVM intel asm is bugged (no way to reference ip, no way to do diff of symbols)
//     naked_asm!(
//         "push %ebx",
//         "call 1f",
//         "1:",
//         "popl %ebx",
//         "addl $_GLOBAL_OFFSET_TABLE_+[.-1b], %ebx",
//         "mov stack_top@GOT(%ebx), %ebx",
//         "xchg %ebx, %esp",
//         "push (%ebx)",
//         "push 4(%ebx)",
//         "push 8(%ebx)",
//         "push 12(%ebx)",
//         "pusha",
//         "mov %esp, %eax",
//         "sub $0x8, %esp",
//         "and $0xFFFFFFF0, %esp",
//         "add $0x8, %esp",
//         "push %eax",
//         "call {irq_stage2}",
//         irq_stage2 = sym irq_stage2,
//         options(att_syntax)
//     );
// }
