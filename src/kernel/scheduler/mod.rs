#[repr(u8)]
#[derive(Copy, Clone, PartialEq)]
enum TaskState {
    Ready,
    Blocked,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Registers {
    rsp: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
    rip: u64,
    rflags: u64,
}
#[repr(C)]
#[derive(Copy, Clone)]
struct Task {
    regs: Registers,
    priority: u8,
    state: TaskState,
    stack: *mut u8,
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

unsafe extern "C" {
    fn context_switch(task: *mut Task);
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
                    SCHEDULER.current_slot[priority] = (slot + 1) % MAX_TASKS_PER_PRIORITY;
                    context_switch(ptr);
                    return;
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
                    SCHEDULER.current_slot[priority] = (slot + 1) % MAX_TASKS_PER_PRIORITY;
                    context_switch(ptr);
                    return;
                }
            }
        }
        return;
    }
}
