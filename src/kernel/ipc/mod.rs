use core::usize;

use crate::{
    cpu,
    scheduler::{self, Task, TaskState, wake},
    serial,
};

const MAX_IPC_MSG_LEN: usize = 4096;
pub const MAX_IPC_CONNECTIONS_PER_TASK: usize = 32;

const IPC_POOL_SIZE: usize = 128;
const EMPTY_MSG: IPCMessage = IPCMessage {
    data: [0u8; MAX_IPC_MSG_LEN],
    len: 0,
};
pub static mut IPC_POOL: [IPCMessage; IPC_POOL_SIZE] = [EMPTY_MSG; IPC_POOL_SIZE];
pub static mut IPC_POOL_USED: [bool; IPC_POOL_SIZE] = [false; IPC_POOL_SIZE];

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
    pub ipc_pool_idx: i32,
    pub has_msg: bool,
}

impl Default for IPCConnection {
    fn default() -> Self {
        IPCConnection {
            peer_pid: -1,
            state: IPCState::PeerDead,
            ipc_pool_idx: -1,
            has_msg: false,
        }
    }
}

pub const ARENA_VADDR: u64 = 0x5000_0000;
pub const ARENA_PAGES: u64 = 16;
pub static mut ARENA_PHYS: u64 = 0;

pub fn init() {
    unsafe {
        let phys = crate::pmm::alloc_pages(4) as u64; // order 4 = 16 contiguous pages
        core::ptr::write_bytes(
            crate::vmm::phys_to_virt(phys) as *mut u8,
            0,
            (ARENA_PAGES * 4096) as usize,
        );
        ARENA_PHYS = phys;
    }
}

pub fn map_arena(pml4: *mut u64) {
    unsafe {
        for i in 0..ARENA_PAGES {
            crate::vmm::map_page(
                pml4,
                ARENA_VADDR + i * 0x1000,
                ARENA_PHYS + i * 0x1000,
                0x07,
            );
        }
    }
}

pub fn alloc_msg() -> i32 {
    for i in 0..IPC_POOL_SIZE {
        unsafe {
            if !IPC_POOL_USED[i] {
                IPC_POOL_USED[i] = true;
                return i as i32;
            }
        }
    }
    return -1;
}

pub fn free_msg(idx: i32) {
    if idx >= 0 {
        unsafe {
            IPC_POOL_USED[idx as usize] = false;
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
    let mut con = IPCConnection::default();
    con.peer_pid = peer_pid;
    con.state = IPCState::Open;
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
        if con.ipc_pool_idx < 0 {
            con.ipc_pool_idx = alloc_msg();
            if con.ipc_pool_idx < 0 {
                return 1;
            }
        }
        let idx = con.ipc_pool_idx as usize;
        let n = core::cmp::min(msg_len as usize, MAX_IPC_MSG_LEN);
        for i in 0..n {
            IPC_POOL[idx].data[i] = *msg.add(i);
        }
        IPC_POOL[idx].len = n;

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
        let msg = core::ptr::addr_of!(IPC_POOL[con.ipc_pool_idx as usize]);
        con.has_msg = false;
        return msg;
    }
}

pub fn recv_any(task: *mut Task, msg_out: *mut IPCMessage) -> i32 {
    unsafe {
        loop {
            for i in 0..MAX_IPC_CONNECTIONS_PER_TASK {
                if (*task).ipc_con[i].is_some() && (*task).ipc_con[i].unwrap().has_msg {
                    let idx = (*task).ipc_con[i].unwrap().ipc_pool_idx;
                    *msg_out = IPC_POOL[idx as usize];
                    (*task).ipc_con[i].as_mut().unwrap().has_msg = false;
                    return i as i32;
                }
            }
            scheduler::block_current();
        }
    }
}
