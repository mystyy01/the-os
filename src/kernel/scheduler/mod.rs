use crate::cpu;
use crate::ipc;
use crate::vmm;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering, fence};

const KSTACK_ORDER: usize = 4;
const KSTACK_SIZE: u64 = 4096 << KSTACK_ORDER;

use crate::{
    cpu::{get_current_task, set_current_task, set_stack_top},
    gdt,
    ipc::{IPCConnection, MAX_IPC_CONNECTIONS_PER_TASK},
    pmm, serial,
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
    pub wake_pending: bool,
    pub stack: *mut u8,
    pub ksp: u64,
    pub kstack_top: u64,
    pub cr3: u64,
    pub pid: i32,
    pub ipc_con: [Option<IPCConnection>; MAX_IPC_CONNECTIONS_PER_TASK],
    pub dma_bump_offset: u64,
}

pub const MAX_TASKS_PER_PRIORITY: usize = 16;
pub const PRIORITY_LEVELS: usize = 8;

#[derive(Copy, Clone)]
struct Scheduler {
    queues: [[Option<Task>; MAX_TASKS_PER_PRIORITY]; PRIORITY_LEVELS],
    current_slot: [usize; PRIORITY_LEVELS],
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}

const MAX_CPUS: usize = 8;

static mut SCHEDULERS: [Scheduler; MAX_CPUS] = [Scheduler {
    queues: [[None; MAX_TASKS_PER_PRIORITY]; PRIORITY_LEVELS],
    current_slot: [0; PRIORITY_LEVELS],
}; MAX_CPUS];

fn next_pid() -> i32 {
    let mut best_pid: i32 = 0;
    unsafe {
        for cpu in 0..MAX_CPUS {
            for priority in 0..PRIORITY_LEVELS {
                for slot in 0..MAX_TASKS_PER_PRIORITY {
                    if let Some(task) = SCHEDULERS[cpu].queues[priority][slot].as_mut() {
                        if task.pid >= best_pid {
                            best_pid = task.pid + 1;
                        }
                    }
                }
            }
        }
    }
    return best_pid;
}

pub fn find_task_by_pid(pid: i32) -> *mut Task {
    unsafe {
        for cpu in 0..MAX_CPUS {
            for priority in 0..PRIORITY_LEVELS {
                for slot in 0..MAX_TASKS_PER_PRIORITY {
                    if let Some(task) = SCHEDULERS[cpu].queues[priority][slot].as_mut() {
                        if task.pid == pid {
                            return task;
                        }
                    }
                }
            }
        }
    }
    return null_mut();
}
pub fn find_ipc_waiting(pid: i32) -> *mut Task {
    unsafe {
        for cpu in 0..MAX_CPUS {
            for priority in 0..PRIORITY_LEVELS {
                for slot in 0..MAX_TASKS_PER_PRIORITY {
                    if let Some(task) = SCHEDULERS[cpu].queues[priority][slot].as_mut() {
                        if task.pid == pid && task.state == TaskState::Blocked {
                            return task;
                        }
                    }
                }
            }
        }
    }
    return null_mut();
}
unsafe fn find_next_task() -> Option<*mut Task> {
    unsafe {
        let s = &mut SCHEDULERS[crate::cpu::id() as usize];
        for priority in (0..PRIORITY_LEVELS).rev() {
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (s.current_slot[priority] + i + 1) % MAX_TASKS_PER_PRIORITY;
                if s.queues[priority][slot].is_some()
                    && s.queues[priority][slot].as_ref().unwrap().state == TaskState::Ready
                {
                    s.current_slot[priority] = slot;
                    return Some(s.queues[priority][slot].as_mut().unwrap() as *mut Task);
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
        (*prev).state = TaskState::Blocked;
        fence(Ordering::SeqCst);
        if core::ptr::read_volatile(&(*prev).wake_pending) {
            core::ptr::write_volatile(&mut (*prev).wake_pending, false);
            (*prev).state = TaskState::Ready;
            return;
        }
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

static WAKE_HINTS: [AtomicPtr<Task>; MAX_CPUS] = [const { AtomicPtr::new(null_mut()) }; MAX_CPUS];

static DIRECT_WAKE_ON: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);

pub fn set_direct_wake(on: bool) {
    DIRECT_WAKE_ON.store(on, Ordering::Relaxed);
}

pub fn set_wake_hint(core: usize, task: *mut Task) {
    if core < MAX_CPUS {
        WAKE_HINTS[core].store(task, Ordering::Release);
    }
}

pub fn try_direct_wake() -> bool {
    unsafe {
        let hint = WAKE_HINTS[crate::cpu::id() as usize].swap(null_mut(), Ordering::AcqRel);
        if hint.is_null() {
            return false;
        }
        if !DIRECT_WAKE_ON.load(Ordering::Relaxed) {
            return false;
        }
        if (*hint).state != TaskState::Ready {
            return false;
        }
        let prev = get_current_task();
        if hint == prev {
            return true;
        }
        set_current_task(Some(hint));

        core::arch::asm!("mov cr3, {}", in(reg) (*hint).cr3, options(nostack));
        set_stack_top((*hint).kstack_top);
        gdt::set_rsp0((*hint).kstack_top);

        switch_to(&raw mut (*prev).ksp, (*hint).ksp);
        return true;
    }
}

pub fn find_local_task(pid: i32) -> *mut Task {
    unsafe {
        let s = &mut SCHEDULERS[crate::cpu::id() as usize];
        for priority in 0..PRIORITY_LEVELS {
            for slot in 0..MAX_TASKS_PER_PRIORITY {
                if let Some(task) = s.queues[priority][slot].as_mut() {
                    if task.pid == pid {
                        return task;
                    }
                }
            }
        }
    }
    return null_mut();
}

pub fn handoff_to(next: *mut Task) -> bool {
    unsafe {
        if next.is_null() {
            return false;
        }
        (*next).state = TaskState::Ready;
        let prev = get_current_task();
        if next == prev {
            return true;
        }
        set_current_task(Some(next));

        core::arch::asm!("mov cr3, {}", in(reg) (*next).cr3, options(nostack));
        set_stack_top((*next).kstack_top);
        gdt::set_rsp0((*next).kstack_top);

        switch_to(&raw mut (*prev).ksp, (*next).ksp);
        return true;
    }
}

pub fn wake(task: Option<*mut Task>) {
    unsafe {
        if task.is_none() {
            return;
        }
        let t = task.unwrap();
        core::ptr::write_volatile(&mut (*t).wake_pending, true);
        fence(Ordering::SeqCst);
        (*t).state = TaskState::Ready;
    }
}

pub fn spawn_task(entry: fn(), priority: u8) {
    unsafe {
        let kstack_phys = pmm::alloc_pages(KSTACK_ORDER) as u64;
        let top = (crate::vmm::phys_to_virt(kstack_phys) + KSTACK_SIZE) as *mut u64;
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
            wake_pending: false,
            priority: priority,
            stack: kstack_phys as *mut u8,
            ksp: ksp,
            kstack_top: top as u64,
            cr3: cr3,
            pid: next_pid(),
            ipc_con: [None; MAX_IPC_CONNECTIONS_PER_TASK],
            dma_bump_offset: 0,
        };
        let s = &mut SCHEDULERS[crate::cpu::id() as usize];
        for (i, t) in s.queues[priority as usize].iter().enumerate() {
            if t.is_none() {
                s.queues[priority as usize][i] = Some(task);
                break;
            }
        }
    }
}

static mut IDLE_PID: i32 = -1;

pub fn spawn_idle(entry: fn()) {
    unsafe {
        let kstack_phys = pmm::alloc_pages(KSTACK_ORDER) as u64;
        let top = (crate::vmm::phys_to_virt(kstack_phys) + KSTACK_SIZE) as *mut u64;
        *top.sub(1) = entry as u64;
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
            wake_pending: false,
            priority: 0,
            stack: kstack_phys as *mut u8,
            ksp: ksp,
            kstack_top: top as u64,
            cr3: cr3,
            pid: IDLE_PID,
            ipc_con: [None; MAX_IPC_CONNECTIONS_PER_TASK],
            dma_bump_offset: 0,
        };
        let s = &mut SCHEDULERS[crate::cpu::id() as usize];
        for (i, t) in s.queues[0 as usize].iter().enumerate() {
            if t.is_none() {
                s.queues[0 as usize][i] = Some(task);
                break;
            }
        }
        IDLE_PID -= 1;
    }
}

pub fn spawn_user_task(entry: u64, user_stack_top: u64, cr3: u64, priority: u8, cpu_id: u8) -> i32 {
    unsafe {
        let kstack_phys = pmm::alloc_pages(KSTACK_ORDER) as u64;
        let top = (crate::vmm::phys_to_virt(kstack_phys) + KSTACK_SIZE) as *mut u64;

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
            wake_pending: false,
            priority: priority,
            stack: kstack_phys as *mut u8,
            ksp: ksp,
            kstack_top: top as u64,
            cr3: cr3,
            pid: next_pid(),
            ipc_con: [None; MAX_IPC_CONNECTIONS_PER_TASK],
            dma_bump_offset: 0,
        };

        let s = &mut SCHEDULERS[cpu_id as usize];
        for (i, t) in s.queues[priority as usize].iter().enumerate() {
            if t.is_none() {
                s.queues[priority as usize][i] = Some(task);
                break;
            }
        }
        return task.pid;
    }
}

pub unsafe fn start() {
    unsafe {
        let s = &mut SCHEDULERS[crate::cpu::id() as usize];
        for priority in (0..PRIORITY_LEVELS).rev() {
            for slot in 0..MAX_TASKS_PER_PRIORITY {
                if s.queues[priority][slot].is_some()
                    && s.queues[priority][slot].as_ref().unwrap().state == TaskState::Ready
                {
                    let first = s.queues[priority][slot].as_mut().unwrap() as *mut Task;
                    set_current_task(Some(first));
                    s.current_slot[priority] = slot;

                    core::arch::asm!("mov cr3, {}", in(reg) (*first).cr3, options(nostack));
                    set_stack_top((*first).kstack_top);
                    gdt::set_rsp0((*first).kstack_top);

                    let mut dummy = 0u64;
                    // core::arch::asm!("swapgs");
                    switch_to(&raw mut dummy, (*first).ksp);
                    return;
                }
            }
        }
    }
}

pub unsafe fn kill_task(task: *mut Task) {
    unsafe {
        let s = &mut SCHEDULERS[crate::cpu::id() as usize];
        for priority in (0..PRIORITY_LEVELS).rev() {
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (s.current_slot[priority] + i) % MAX_TASKS_PER_PRIORITY;
                if s.queues[priority][slot].is_some()
                    && s.queues[priority][slot].as_mut().unwrap() as *mut Task == task
                {
                    s.queues[priority][slot] = None;
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

pub unsafe fn kill_current_task() {
    unsafe {
        kill_task(get_current_task());
    }
}

pub unsafe fn cleanup_and_exit_task(task: *mut Task) {
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) cpu::get_kernel_cr3());
        vmm::free_table((*task).cr3, 4);
        if !(*task).stack.is_null() {
            pmm::free_pages(KSTACK_ORDER, (*task).stack as u64);
        }

        for con in (*task).ipc_con {
            if con.is_some() {
                ipc::free_msg(con.unwrap().ipc_pool_idx);
            }
        }

        ipc::release_server_by_pid((*task).pid);

        kill_task(task);
    }
}
