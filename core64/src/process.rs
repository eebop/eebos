use alloc::boxed::Box;
use alloc::vec::Vec;
use shared::*;

use core::arch::asm;

struct Process {
    got_ptr: u32,
    stacks: Vec<Box<[u8]>>, // Unitialized
}

unsafe extern "C" {
    static mut stored_sp: u32;
    static mut stack_top: u8;
}

impl Process {
    fn new_stack(&mut self) {
        let x = Box::<[u8]>::new_uninit_slice(16384);
        let x = unsafe { x.assume_init() };
        self.stacks.push(x);

    }
    fn make_start(&mut self, _start: extern "C" fn()) -> ! {
        // TODO: Initialization is more complex if you have argv that isn't empty
        self.new_stack();
        let size = self.stacks.last().unwrap().len();
        self.stacks.last_mut().unwrap()[size-16..size].fill(0); // this sets up argv, argc, envp. all 0
        
        let ptr = &raw mut self.stacks.last_mut().unwrap()[size - 16];




        // ESP must point to the top of our stack
        // EDX must point to the atexit() function
        // unsafe { asm!(
            
        //     "call {_start}",
        //     _start = in(reg) _start,

        // )}
    }

    fn make_interrupt(&mut self, fnptr: extern "C" fn(&mut SysCallData, &mut State), sys: &mut SysCallData, state: &mut State) {

    }
}