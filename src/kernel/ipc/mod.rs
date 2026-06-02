use core::ptr::null;

use crate::{
    cpu,
    scheduler::{self, Task, TaskState, wake},
    serial,
};

const MAX_IPC_MSG_LEN: usize = 256;

#[derive(Clone, Copy)]
pub struct IPCMessage {
    pub sender_pid: i32,
    pub data: [u8; MAX_IPC_MSG_LEN],
    pub len: usize,
}

unsafe impl Send for IPCMessage {}
unsafe impl Sync for IPCMessage {}

impl Default for IPCMessage {
    fn default() -> Self {
        IPCMessage {
            sender_pid: -1,
            data: [0 as u8; MAX_IPC_MSG_LEN],
            len: MAX_IPC_MSG_LEN,
        }
    }
}

pub fn write_ipc(task: *mut Task, msg: *const u8, msg_len: i32) -> i32 {
    unsafe {
        serial::write_str("IPC sent to: ");
        serial::write_hex((*task).pid as u64);
        serial::write_str("\nwith message: ");
        for i in 0..msg_len {
            serial::write_byte(*msg.add(i as usize));
        }

        let dst = &mut (*task).ipc_msg;
        dst.sender_pid = (*cpu::get_current_task()).pid;
        let n = core::cmp::min(msg_len as usize, MAX_IPC_MSG_LEN);
        for i in 0..n {
            dst.data[i] = *msg.add(i);
        }
        dst.len = n as usize;

        if (*task).state == TaskState::Blocked {
            wake(Some(task));
        }

        return 0;
    }
}

pub fn read_ipc(task: *mut Task) -> *const IPCMessage {
    unsafe {
        if (*task).ipc_msg.sender_pid < 0 {
            // we know theres no ipc message being sent
            scheduler::block_current();
            // block the task and let the write wake it up i think
            return &(*task).ipc_msg;
        }
        let msg = &(*task).ipc_msg;
        (*task).ipc_msg.sender_pid = -1;
        return msg;
    }
}
