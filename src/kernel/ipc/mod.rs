use crate::{
    cpu,
    scheduler::{self, Task, TaskState, wake},
    serial,
};

const MAX_IPC_MSG_LEN: usize = 256;

#[derive(Clone, Copy)]
pub struct IPCMessage {
    pub data: [u8; MAX_IPC_MSG_LEN],
    pub len: usize,
}

unsafe impl Send for IPCMessage {}
unsafe impl Sync for IPCMessage {}

impl Default for IPCMessage {
    fn default() -> Self {
        IPCMessage {
            data: [0 as u8; MAX_IPC_MSG_LEN],
            len: MAX_IPC_MSG_LEN,
        }
    }
}

#[derive(Clone, Copy)]
pub enum IPCState {
    Open,
    AwaitingReply,
    PeerDead,
}

#[derive(Clone, Copy)]
pub struct IPCConnection {
    pub peer_pid: i32,
    pub state: IPCState,
    pub msg: IPCMessage,
    pub has_msg: bool,
}

impl Default for IPCConnection {
    fn default() -> Self {
        IPCConnection {
            peer_pid: -1,
            state: IPCState::PeerDead,
            msg: IPCMessage::default(),
            has_msg: false,
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

        let sender = (*cpu::get_current_task()).pid;
        if (*task).state == TaskState::Blocked {
            match (*task).ipc_con.state {
                IPCState::AwaitingReply => {
                    if sender != (*task).ipc_con.peer_pid {
                        return 1;
                    }
                }
                _ => {}
            }
        }

        let dst = &mut (*task).ipc_con;
        dst.peer_pid = (*cpu::get_current_task()).pid;
        let n = core::cmp::min(msg_len as usize, MAX_IPC_MSG_LEN);
        for i in 0..n {
            dst.msg.data[i] = *msg.add(i);
        }
        dst.msg.len = n as usize;
        dst.has_msg = true;

        if (*task).state == TaskState::Blocked {
            wake(Some(task));
        }

        return 0;
    }
}

pub fn read_ipc(task: *mut Task) -> *const IPCMessage {
    unsafe {
        while !(*task).ipc_con.has_msg {
            // we know theres no ipc message being sent
            scheduler::block_current();
            // block the task and let the write wake it up i think
        }
        let msg = &(*task).ipc_con.msg;
        (*task).ipc_con.has_msg = false;
        return msg;
    }
}
