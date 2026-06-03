use crate::{
    ipc::{alloc_con, conn_mut, find_conn_to},
    scheduler,
};

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
        let ipcd = match find_conn_to(task, 0) {
            Some(ipcd) => ipcd,
            None => alloc_con(task, 0),
        };
        if ipcd < 0 {
            return;
        }
        let con = conn_mut(task, ipcd).unwrap();
        con.msg.len = 0;
        con.has_msg = true;
        scheduler::wake(Some(task));
    }
}
