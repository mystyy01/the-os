use core::ptr::null_mut;

use crate::{
    cpu::{get_current_task, set_current_task, set_stack_top},
    gdt,
    ipc::IPCMessage,
    pmm,
};

unsafe extern "C" {
    pub fn switch_to(prev: *mut u64, next: u64);
    pub fn user_entry_bouncy_trampoline_lol();
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq)]
pub enum TaskState {
    Ready,
    Blocked,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Task {
    pub priority: u8,
    pub state: TaskState,
    pub stack: *mut u8,
    pub ksp: u64,
    kstack_top: u64,
    pub cr3: u64,
    pub pid: i32,
    pub pid_waiting_ipc: i32,
    pub ipc_msg: IPCMessage,
}

pub const MAX_TASKS_PER_PRIORITY: usize = 16;
pub const PRIORITY_LEVELS: usize = 8;

struct Scheduler {
    queues: [[Option<Task>; MAX_TASKS_PER_PRIORITY]; PRIORITY_LEVELS],
    current_slot: [usize; PRIORITY_LEVELS],
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}

static mut SCHEDULER: Scheduler = Scheduler {
    queues: [[None; MAX_TASKS_PER_PRIORITY]; PRIORITY_LEVELS],
    current_slot: [0; PRIORITY_LEVELS],
};

fn next_pid() -> i32 {
    let mut best_pid: i32 = 0;
    unsafe {
        for priority in 0..PRIORITY_LEVELS {
            for slot in 0..MAX_TASKS_PER_PRIORITY {
                if let Some(task) = SCHEDULER.queues[priority][slot].as_mut() {
                    if task.pid >= best_pid {
                        best_pid += 1;
                    }
                }
            }
        }
    }
    return best_pid;
}

pub fn find_task_by_pid(pid: i32) -> *mut Task {
    unsafe {
        for priority in 0..PRIORITY_LEVELS {
            for slot in 0..MAX_TASKS_PER_PRIORITY {
                if let Some(task) = SCHEDULER.queues[priority][slot].as_mut() {
                    if task.pid == pid {
                        return task;
                    }
                }
            }
        }
    }
    return null_mut();
}
pub fn find_ipc_waiting(pid: i32) -> *mut Task {
    unsafe {
        for priority in 0..PRIORITY_LEVELS {
            for slot in 0..MAX_TASKS_PER_PRIORITY {
                if let Some(task) = SCHEDULER.queues[priority][slot].as_mut() {
                    if task.pid == pid && task.state == TaskState::Blocked {
                        return task;
                    }
                }
            }
        }
    }
    return null_mut();
}
unsafe fn find_next_task() -> Option<*mut Task> {
    unsafe {
        for priority in (0..PRIORITY_LEVELS).rev() {
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (SCHEDULER.current_slot[priority] + i + 1) % MAX_TASKS_PER_PRIORITY;
                if SCHEDULER.queues[priority][slot].is_some()
                    && SCHEDULER.queues[priority][slot].as_ref().unwrap().state == TaskState::Ready
                {
                    SCHEDULER.current_slot[priority] = slot;
                    return Some(SCHEDULER.queues[priority][slot].as_mut().unwrap() as *mut Task);
                }
            }
        }
        return None;
    }
}
pub fn yield_now() {
    unsafe {
        let prev = get_current_task();
        if let Some(next) = find_next_task() {
            if next == prev {
                return;
            }
            set_current_task(Some(next));

            core::arch::asm!("mov cr3, {}", in(reg) (*next).cr3, options(nostack));
            set_stack_top((*next).kstack_top);
            gdt::set_rsp0((*next).kstack_top);

            switch_to(&raw mut (*prev).ksp, (*next).ksp);
        }
    }
}

pub fn block_current() {
    unsafe {
        let prev = get_current_task();
        if let Some(next) = find_next_task() {
            if next == prev {
                return;
            }
            (*prev).state = TaskState::Blocked;
            set_current_task(Some(next));
            core::arch::asm!("mov cr3, {}", in(reg) (*next).cr3, options(nostack));
            set_stack_top((*next).kstack_top);
            gdt::set_rsp0((*next).kstack_top);

            switch_to(&raw mut (*prev).ksp, (*next).ksp);
        }
    }
}

pub fn wake(task: Option<*mut Task>) {
    unsafe {
        if task.is_none() {
            return;
        }
        (*task.unwrap()).state = TaskState::Ready;
    }
}

pub fn spawn_task(entry: fn(), priority: u8) {
    let stack = crate::pmm::alloc_pages(2) as *mut u8;
    unsafe {
        let kstack_phys = pmm::alloc_pages(1) as u64;
        let top = (crate::vmm::phys_to_virt(kstack_phys) + 8192) as *mut u64;
        *top.sub(1) = entry as u64; // ret lands here
        *top.sub(2) = 0;
        *top.sub(3) = 0;
        *top.sub(4) = 0;
        *top.sub(5) = 0;
        *top.sub(6) = 0;
        *top.sub(7) = 0;
        let ksp = top.sub(7) as u64;

        let mut cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        let task = Task {
            state: TaskState::Ready,
            priority: priority,
            stack: stack,
            ksp: ksp,
            kstack_top: top as u64,
            cr3: cr3,
            pid: next_pid(),
            ipc_msg: IPCMessage::default(),
            pid_waiting_ipc: -1,
        };
        for (i, t) in SCHEDULER.queues[priority as usize].iter().enumerate() {
            if t.is_none() {
                SCHEDULER.queues[priority as usize][i] = Some(task);
                break;
            }
        }
    }
}

pub fn spawn_user_task(entry: u64, user_stack_top: u64, cr3: u64, priority: u8) {
    let stack = crate::pmm::alloc_pages(2) as *mut u8;
    unsafe {
        let kstack_phys = pmm::alloc_pages(1) as u64;
        let top = (crate::vmm::phys_to_virt(kstack_phys) + 8192) as *mut u64;

        *top.sub(1) = user_entry_bouncy_trampoline_lol as u64;
        *top.sub(2) = 0;
        *top.sub(3) = 0;
        *top.sub(4) = 0;
        *top.sub(5) = 0;
        *top.sub(6) = user_stack_top;
        *top.sub(7) = entry;
        let ksp = top.sub(7) as u64;

        let task = Task {
            state: TaskState::Ready,
            priority: priority,
            stack: null_mut(),
            ksp: ksp,
            kstack_top: top as u64,
            cr3: cr3,
            pid: next_pid(),
            ipc_msg: IPCMessage::default(),
            pid_waiting_ipc: -1,
        };
        for (i, t) in SCHEDULER.queues[priority as usize].iter().enumerate() {
            if t.is_none() {
                SCHEDULER.queues[priority as usize][i] = Some(task);
                break;
            }
        }
    }
}

pub unsafe fn start() {
    unsafe {
        for priority in (0..PRIORITY_LEVELS).rev() {
            for slot in 0..MAX_TASKS_PER_PRIORITY {
                if SCHEDULER.queues[priority][slot].is_some()
                    && SCHEDULER.queues[priority][slot].as_ref().unwrap().state == TaskState::Ready
                {
                    let first = SCHEDULER.queues[priority][slot].as_mut().unwrap() as *mut Task;
                    set_current_task(Some(first));
                    SCHEDULER.current_slot[priority] = slot;

                    core::arch::asm!("mov cr3, {}", in(reg) (*first).cr3, options(nostack));
                    set_stack_top((*first).kstack_top);
                    gdt::set_rsp0((*first).kstack_top);

                    let mut dummy = 0u64;
                    core::arch::asm!("swapgs");
                    switch_to(&raw mut dummy, (*first).ksp);
                    return;
                }
            }
        }
    }
}

pub unsafe fn kill_current_task() {
    unsafe {
        for priority in (0..PRIORITY_LEVELS).rev() {
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (SCHEDULER.current_slot[priority] + i) % MAX_TASKS_PER_PRIORITY;
                if SCHEDULER.queues[priority][slot].is_some()
                    && SCHEDULER.queues[priority][slot].as_mut().unwrap() as *mut Task
                        == get_current_task()
                {
                    SCHEDULER.queues[priority][slot] = None;
                    break;
                }
            }
        }

        if let Some(next) = find_next_task() {
            set_current_task(Some(next));
            core::arch::asm!("mov cr3, {}", in(reg) (*next).cr3, options(nostack));
            set_stack_top((*next).kstack_top);
            gdt::set_rsp0((*next).kstack_top);

            let mut dummy = 0u64;
            switch_to(&raw mut dummy, (*next).ksp);
        }
    }
}
