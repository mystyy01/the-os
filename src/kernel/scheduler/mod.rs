use crate::cpu::set_current_task;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq)]
enum TaskState {
    Ready,
    Blocked,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Registers {
    pub rsp: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u64,
    pub ss: u64,
    pub cr3: u64,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Task {
    pub regs: Registers,
    pub priority: u8,
    pub state: TaskState,
    pub stack: *mut u8,
}

pub const MAX_TASKS_PER_PRIORITY: usize = 16;
pub const PRIORITY_LEVELS: usize = 8;

struct Scheduler {
    queues: [[Option<Task>; MAX_TASKS_PER_PRIORITY]; PRIORITY_LEVELS],
    current: Option<*mut Task>,
    current_slot: [usize; PRIORITY_LEVELS],
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}

static mut SCHEDULER: Scheduler = Scheduler {
    queues: [[None; MAX_TASKS_PER_PRIORITY]; PRIORITY_LEVELS],
    current: None,
    current_slot: [0; PRIORITY_LEVELS],
};

pub fn spawn_task(entry: fn(), priority: u8) {
    let stack = crate::pmm::alloc_pages(2) as *mut u8;
    unsafe {
        let stack_top = stack.add(4096 * 4);
        let mut cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        let regs = Registers {
            rsp: stack_top as u64,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbp: 0,
            rdi: 0,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            rip: entry as u64,
            rflags: 0x202,
            cs: 0x08,
            ss: 0x10,
            cr3: cr3,
        };
        let task = Task {
            regs: regs,
            state: TaskState::Ready,
            priority: priority,
            stack: stack,
        };
        for (i, t) in SCHEDULER.queues[priority as usize].iter().enumerate() {
            if t.is_none() {
                SCHEDULER.queues[priority as usize][i] = Some(task);
                break;
            }
        }
    }
}

pub fn spawn_user_task(entry: u64, stack_top: u64, cr3: u64, priority: u8) {
    unsafe {
        let regs = Registers {
            rsp: stack_top as u64,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbp: 0,
            rdi: 0,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            rip: entry as u64,
            rflags: 0x202,
            cs: 0x23,
            ss: 0x1b,
            cr3: cr3,
        };
        let task = Task {
            regs: regs,
            state: TaskState::Ready,
            priority: priority,
            stack: core::ptr::null_mut(),
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
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (SCHEDULER.current_slot[priority] + i) % MAX_TASKS_PER_PRIORITY;
                if SCHEDULER.queues[priority][slot].is_some()
                    && SCHEDULER.queues[priority][slot].as_ref().unwrap().state == TaskState::Ready
                {
                    let ptr = SCHEDULER.queues[priority][slot].as_mut().unwrap() as *mut Task;
                    SCHEDULER.current = Some(ptr);
                    set_current_task(Some(ptr));

                    core::arch::asm!("mov cr3, {}", in(reg) (*ptr).regs.cr3, options(nostack));
                    core::arch::asm!(
                        "push {ss}",
                        "push {rsp}",
                        "push {rflags}",
                        "push {cs}",
                        "push {rip}",
                        "iretq",
                        ss = in(reg) (*ptr).regs.ss,
                        rsp = in(reg) (*ptr).regs.rsp,
                        rflags = in(reg) (*ptr).regs.rflags,
                        cs = in(reg) (*ptr).regs.cs,
                        rip = in(reg) (*ptr).regs.rip,
                        options(noreturn),
                    );
                }
            }
        }
        return;
    }
}

pub unsafe fn kill_current_task() {
    unsafe {
        for priority in (0..PRIORITY_LEVELS).rev() {
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (SCHEDULER.current_slot[priority] + i) % MAX_TASKS_PER_PRIORITY;
                if SCHEDULER.queues[priority][slot].is_some()
                    && SCHEDULER.queues[priority][slot].as_mut().unwrap() as *mut Task
                        == SCHEDULER.current.unwrap()
                {
                    SCHEDULER.queues[priority][slot] = None;
                    break;
                }
            }
        }

        SCHEDULER.current = None;
        set_current_task(None);

        for priority in (0..PRIORITY_LEVELS).rev() {
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (SCHEDULER.current_slot[priority] + i) % MAX_TASKS_PER_PRIORITY;
                if SCHEDULER.queues[priority][slot].is_some()
                    && SCHEDULER.queues[priority][slot].as_ref().unwrap().state == TaskState::Ready
                {
                    let ptr = SCHEDULER.queues[priority][slot].as_mut().unwrap() as *mut Task;
                    SCHEDULER.current = Some(ptr);
                    set_current_task(Some(ptr));

                    core::arch::asm!("mov cr3, {}", in(reg) (*ptr).regs.cr3, options(nostack));
                    core::arch::asm!(
                        "push {ss}",
                        "push {rsp}",
                        "push {rflags}",
                        "push {cs}",
                        "push {rip}",
                        "iretq",
                        ss = in(reg) (*ptr).regs.ss,
                        rsp = in(reg) (*ptr).regs.rsp,
                        rflags = in(reg) (*ptr).regs.rflags,
                        cs = in(reg) (*ptr).regs.cs,
                        rip = in(reg) (*ptr).regs.rip,
                        options(noreturn),
                    );
                }
            }
        }
        return;
    }
}

pub unsafe fn schedule(frame: *mut u64) {
    unsafe {
        if let Some(save) = SCHEDULER.current {
            (*save).regs.rip = *frame.add(17);
            (*save).regs.rsp = *frame.add(20);
            (*save).regs.rflags = *frame.add(19);

            (*save).regs.rax = *frame.add(14);
            (*save).regs.rbx = *frame.add(13);
            (*save).regs.rcx = *frame.add(12);
            (*save).regs.rdx = *frame.add(11);
            (*save).regs.rsi = *frame.add(10);
            (*save).regs.rdi = *frame.add(9);
            (*save).regs.rbp = *frame.add(8);
            (*save).regs.r8 = *frame.add(7);
            (*save).regs.r9 = *frame.add(6);
            (*save).regs.r10 = *frame.add(5);
            (*save).regs.r11 = *frame.add(4);
            (*save).regs.r12 = *frame.add(3);
            (*save).regs.r13 = *frame.add(2);
            (*save).regs.r14 = *frame.add(1);
            (*save).regs.r15 = *frame.add(0);
        }
        for priority in (0..PRIORITY_LEVELS).rev() {
            for i in 0..MAX_TASKS_PER_PRIORITY {
                let slot = (SCHEDULER.current_slot[priority] + i) % MAX_TASKS_PER_PRIORITY;
                if SCHEDULER.queues[priority][slot].is_some()
                    && SCHEDULER.queues[priority][slot].as_ref().unwrap().state == TaskState::Ready
                {
                    let ptr = SCHEDULER.queues[priority][slot].as_mut().unwrap() as *mut Task;

                    SCHEDULER.current = Some(ptr);
                    set_current_task(Some(ptr));
                    SCHEDULER.current_slot[priority] = (slot + 1) % MAX_TASKS_PER_PRIORITY;
                    *frame.add(0) = (*ptr).regs.r15;
                    *frame.add(1) = (*ptr).regs.r14;
                    *frame.add(2) = (*ptr).regs.r13;
                    *frame.add(3) = (*ptr).regs.r12;
                    *frame.add(4) = (*ptr).regs.r11;
                    *frame.add(5) = (*ptr).regs.r10;
                    *frame.add(6) = (*ptr).regs.r9;
                    *frame.add(7) = (*ptr).regs.r8;
                    *frame.add(8) = (*ptr).regs.rbp;
                    *frame.add(9) = (*ptr).regs.rdi;
                    *frame.add(10) = (*ptr).regs.rsi;
                    *frame.add(11) = (*ptr).regs.rdx;
                    *frame.add(12) = (*ptr).regs.rcx;
                    *frame.add(13) = (*ptr).regs.rbx;
                    *frame.add(14) = (*ptr).regs.rax;
                    *frame.add(17) = (*ptr).regs.rip;
                    *frame.add(18) = (*ptr).regs.cs;
                    *frame.add(19) = (*ptr).regs.rflags;
                    *frame.add(20) = (*ptr).regs.rsp;
                    *frame.add(21) = (*ptr).regs.ss;
                    core::arch::asm!("mov cr3, {}", in(reg) (*ptr).regs.cr3, options(nostack));
                    return;
                }
            }
        }
        return;
    }
}
