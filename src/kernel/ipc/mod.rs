use crate::{
    cpu,
    scheduler::{self, Task, TaskState, wake},
    serial,
};

const MAX_IPC_MSG_LEN: usize = 256;
pub const MAX_IPC_CONNECTIONS_PER_TASK: usize = 32;

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

pub fn find_free_ipcd(task: *mut Task) -> i32 {
    unsafe {
        for i in 0..MAX_IPC_CONNECTIONS_PER_TASK {
            if (*task).ipc_con[i].is_some() {
                continue;
            }
            // now i is free ipcd
            return i as i32;
        }
    }
    return -1;
}

pub fn alloc_con(task: *mut Task, peer_pid: i32) -> i32 {
    let ipcd = find_free_ipcd(task);
    if ipcd < 0 {
        return -1;
    }
    let con = IPCConnection {
        peer_pid: peer_pid,
        state: IPCState::Open,
        msg: IPCMessage::default(),
        has_msg: false,
    };
    unsafe {
        (*task).ipc_con[ipcd as usize] = Some(con);
    }
    return ipcd;
}

pub fn find_conn_to(task: *mut Task, peer_pid: i32) -> Option<i32> {
    unsafe {
        for i in 0..MAX_IPC_CONNECTIONS_PER_TASK {
            if (*task).ipc_con[i].is_none() {
                continue;
            }
            if (*task).ipc_con[i].unwrap().peer_pid == peer_pid {
                return Some(i as i32);
            }
        }
    }
    return None;
}

pub unsafe fn conn_mut<'a>(task: *mut Task, ipcd: i32) -> Option<&'a mut IPCConnection> {
    if ipcd < 0 || ipcd as usize >= MAX_IPC_CONNECTIONS_PER_TASK {
        return None;
    }
    unsafe {
        return (*task).ipc_con[ipcd as usize].as_mut();
    }
}

pub fn write_ipc(task: *mut Task, ipcd: i32, msg: *const u8, msg_len: i32) -> i32 {
    unsafe {
        if msg_len < 0 {
            return 1;
        }

        serial::write_str("IPC sent to: ");
        serial::write_hex((*task).pid as u64);
        serial::write_str("\nwith message: ");
        for i in 0..msg_len {
            serial::write_byte(*msg.add(i as usize));
        }

        let con = conn_mut(task, ipcd).unwrap();
        let sender = (*cpu::get_current_task()).pid;
        if (*task).state == TaskState::Blocked {
            match con.state {
                IPCState::AwaitingReply => {
                    if sender != con.peer_pid {
                        return 1;
                    }
                }
                _ => {}
            }
        }

        let n = core::cmp::min(msg_len as usize, MAX_IPC_MSG_LEN);
        for i in 0..n {
            con.msg.data[i] = *msg.add(i);
        }
        con.msg.len = n as usize;
        con.has_msg = true;

        if (*task).state == TaskState::Blocked {
            wake(Some(task));
        }

        return 0;
    }
}

pub fn read_ipc(task: *mut Task, ipcd: i32) -> *const IPCMessage {
    unsafe {
        loop {
            let con = conn_mut(task, ipcd).unwrap();
            if con.has_msg {
                break;
            }
            scheduler::block_current();
        }
        let con = conn_mut(task, ipcd).unwrap();
        let msg = core::ptr::addr_of!(con.msg);
        con.has_msg = false;
        return msg;
    }
}

pub fn recv_any(task: *mut Task, msg_out: *mut IPCMessage) -> i32 {
    unsafe {
        for i in 0..MAX_IPC_CONNECTIONS_PER_TASK {
            if (*task).ipc_con[i].is_some() && (*task).ipc_con[i].unwrap().has_msg {
                // this is a uhh message needed to be sent
                *msg_out = (*task).ipc_con[i].unwrap().msg;
                return i as i32;
            }
        }
    }
    return -1;
}
