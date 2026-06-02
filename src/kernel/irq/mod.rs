use crate::scheduler;

static mut IRQ_HANDLERS: [i32; 16] = [-1; 16];

pub fn register(irq: usize, pid: i32) {
    if irq < 16 {
        unsafe {
            IRQ_HANDLERS[irq] = pid;
        }
    }
}

pub fn dispatch(irq: usize) {
    unsafe {
        if irq >= 16 {
            return;
        }
        let pid = IRQ_HANDLERS[irq];
        if pid < 0 {
            return;
        }
        let task = scheduler::find_task_by_pid(pid);
        if task.is_null() {
            return;
        }
        (*task).ipc_msg.sender_pid = 0;
        (*task).ipc_msg.len = 0;
        scheduler::wake(Some(task));
    }
}
