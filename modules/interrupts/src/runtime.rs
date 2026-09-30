use core::{arch::naked_asm, cell::Cell, pin::{Pin, pin}, sync::atomic::{AtomicBool, Ordering::Release}, task::{Context, Waker}};

use alloc::{boxed::Box, collections::VecDeque, sync::{Arc, Weak}, task::Wake};
use dyshared::screen::Screen;
use paging::{PM, PageMap, PageToken};
use core::fmt::Write;

use crate::{idt::{RegState, rejoin_task}, interrupts::{Interrupt, SysInt}, single_threaded::{SingleThreadToken, SingleThreaded}};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskState {
    Running,
    // Waiting for its wakesignal
    Sleeping(Interrupt)
}

struct WakeSignal {
    is_woken: AtomicBool
}

impl WakeSignal {
    fn new() -> Self {
        Self { is_woken: AtomicBool::new(false) }
    }
}

impl Wake for WakeSignal {
    fn wake(self: Arc<Self>) {
        self.is_woken.store(true, Release);
    }
}

pub struct AsyncTask {
    task: Arc<Task>,
    woken: Arc<WakeSignal>
}

impl AsyncTask {
    fn from_async(f: impl Future<Output=()> + Send, stack_top: *mut u32, pages: PM) -> AsyncTask {
        let woken = Arc::new(WakeSignal::new());

        let woken_send = Arc::downgrade(&woken);
        let closure = || {
            let mut f = pin!(f);

            loop {
                let Some(waker) = Weak::upgrade(&woken_send) else {
                    break;
                };
                let waker = Waker::from(waker);
                let mut cx = Context::from_waker(&waker);
                if f.as_mut().poll(&mut cx).is_ready() {
                    break;
                }
                todo!(); // cede time slice
            }
            todo!(); // end task
        };

        let task = Arc::new(Task::from_fn(closure, stack_top, pages));
        Self {
            task,
            woken
        }
    }
}

pub struct Task {
    registers: RegState,
    pages: PM,

    status: SingleThreaded<Cell<TaskState>>,
}

impl Task {

    pub fn from_fn(closure: impl FnOnce() -> ! + Send, stack_top: *mut u32, pages: PM) -> Self {
        let mut  registers = RegState::from_fn(entrypoint, stack_top);

        let closure = Box::new(closure) as Box<dyn FnOnce() -> ! + Send>;
        
        let data = Box::into_raw(closure);
        let data: [u32; 2] = unsafe { core::mem::transmute(data) };

        registers.eax = data[0];
        registers.ebx = data[1];

        Task {
            registers,
            pages,
            status: SingleThreaded::new(TaskState::Running),
        }
    }
}

pub static RT: SingleThreaded<Option<Schedule>> = SingleThreaded::new(None);

pub struct Schedule {
    current: Arc<Task>,
    tasks: VecDeque<Weak<Task>>,
    page_token: Option<PageToken>
}

impl Schedule {
    pub fn new(token: PageToken, start: Arc<Task>) -> Self {
        Self {
            current: start,
            tasks: VecDeque::new(),
            page_token: Some(token)
        }
    }

    pub fn next(&mut self, mut proof: SingleThreadToken, msg: Interrupt) -> RegState {
        *self.current.status.get(&mut proof.reborrow()) = &mut TaskState::Sleeping(msg);

        loop {
            let next = self.tasks.pop_front().unwrap();
            if let Some(next) = next.upgrade() {
                self.current = next;
                self.become_task();
            }
        }
    }

    // pub fn start(&mut self) -> ! {
    //     self.current = self.tasks.pop_front();
    //     self.become_task();

    // }

    // pub fn schedule(&mut self) {
    //     let curr = self.tasks.pop_front().unwrap();
    //     assert!(curr.status == TaskState::Running);
    //     self.become_task()
    // }

    // pub fn insert(&mut self, task: Rc<Task>) {
    //     self.tasks.push_back(task);
    // }

    pub fn become_task(&mut self) -> ! {
        let token = self.page_token.take().unwrap();
        let token = unsafe { self.current.pages.build(token) };
        self.page_token = Some(token);
        let registers = self.current.registers;
        rejoin_task(registers)
    }
}



fn async_entry_stage2(data1: u32, data2: u32) -> ! {
    let data = [data1, data2];
    let data: *mut (dyn FnOnce() -> ! + Send) = unsafe { core::mem::transmute(data) };
    let data = unsafe { Box::from_raw(data) };
    data()
}

#[unsafe(naked)]
extern "C" fn entrypoint() {
    naked_asm!(
        "push ecx",
        "push ebx",
        "push eax",
        "call {async_entry_stage2}",
        "ud2",
        async_entry_stage2 = sym async_entry_stage2
    )
}