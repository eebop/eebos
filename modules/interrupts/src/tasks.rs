use core::{cell::{Cell, RefCell, RefMut}, marker::PhantomData, sync::atomic::AtomicBool};
use core::fmt::Write;

use alloc::{rc::Rc, vec::Vec};
use dyshared::{SimpleAllocator, screen::Screen};
use paging::{PM, PageMap, PageToken};

use crate::{idt::{RegState, init_interrupts, rejoin_task}, single_threaded::SingleThreaded};

pub struct State {
    pages: PM,
    state: RegState,

}

impl State {
    pub fn from_fn(f: extern "C" fn() -> !, mut stack_top: *mut u32, pages: PM) -> State {
        assert!(stack_top.is_aligned_to(16));
        stack_top = unsafe { stack_top.sub(4) }; // 16 bytes are saved for ret
        let state = RegState {
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
        };
        State {pages, state }
        // unsafe { assert!(!STARTED) }; // started -> writing to Tasks is a race
        // unsafe { TASKS.borrow_mut().push(t) };
    }
}


// static mut STARTED: bool = false;
// pub static mut CRITICAL: bool = false;

// static TASKS: SingleThreaded<RefCell<Vec<Rc<RefCell<State>>, SimpleAllocator>>> = unsafe { SingleThreaded::new(RefCell::new(Vec::new_in(SimpleAllocator))) };

// static CURRENT_TASK: SingleThreaded<Cell<Option<Rc<RefCell<State>>>>> = unsafe { SingleThreaded::new(Cell::new(None)) };

// static PAGE_TOKEN: SingleThreaded<Cell<Option<PageToken>>> = unsafe { SingleThreaded::new(Cell::new(None)) };

// pub fn start_scheduler(token: PageToken) -> ! {
//     unsafe {assert!(!STARTED); STARTED = true};

//     let task = TASKS.borrow().first().unwrap().clone();

//     let token = unsafe { task.borrow_mut().pages.build(token) };
//     PAGE_TOKEN.set(Some(token));

//     let state = task.borrow().state;

//     CURRENT_TASK.set(Some(task));

//     // TODO: (IMPORTANT) need to tun off interrupts while doing this

//     init_interrupts(SimpleAllocator);

//     // pic::enable(0);

//     rejoin_task(state)
// }

// pub fn schedule(state: RegState) -> ! {

//     {
//         let bind = CURRENT_TASK.take().unwrap();
//         let mut prev_task = bind.borrow_mut();
//         prev_task.state = state;
//     }

//     let index = {
//         let len = TASKS.borrow().len();

//         unsafe {
//             static mut INDEX: usize = 0;
//             let mut index = INDEX;
//             index = (index + 1) % len;
//             INDEX = index;
//             index
//         }
//     };


//     // writeln!(Screen, "\nNow doing value: {}", index);

//     let task = TASKS.borrow().get(index).unwrap().clone();
//     let token = PAGE_TOKEN.take().unwrap();
//     let token = unsafe { task.borrow_mut().pages.build(token) };
//     PAGE_TOKEN.set(Some(token));

    
//     let state = task.borrow().state;
//     CURRENT_TASK.set(Some(task));
//     // writeln!(Screen, "state is: {:?}", state);
//     // assert!(state.cs == 8);
//     // if state.cs != 8 {
//     // }
//     // pic::sendEOI(0);
//     rejoin_task(state)
// }