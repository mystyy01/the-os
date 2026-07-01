use core::sync::atomic::{Ordering, fence};
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
pub const ARENA_PAGES: u64 = 128;
pub static mut ARENA_PHYS: u64 = 0;

pub const MBOX_OFF: usize = 4096;
pub const BUFPOOL_OFF: usize = 0x10000;
pub const BUF_SIZE: usize = 4096;
pub const MAX_MAILBOXES: usize = 64;
pub const OP_IRQ: u8 = 6;
pub const MBOX_REQ: u32 = 1;

pub const IRQRING_OFF: usize = 0x50000;
pub const IRQRING_CAP: usize = 256;
pub const MAX_IRQ: usize = 16;

#[repr(C)]
pub struct IrqRing {
    pub head: u32,
    pub tail: u32,
    pub data: [u8; IRQRING_CAP],
}

pub const INBOX_OFF: usize = 0x55000;
pub const MAX_SERVICES: usize = 16;
pub const INBOX_CAP: usize = 48;

#[repr(C)]
pub struct ServiceInbox {
    pub count: u32,
    pub _pad: u32,
    pub idx: [u32; INBOX_CAP],
}

#[derive(Copy, Clone)]
pub struct ServerReg {
    pub pid: i32,
    pub core: u32,
    pub used: bool,
}

static mut SERVERS: [ServerReg; MAX_SERVICES] = [ServerReg {
    pid: -1,
    core: 0,
    used: false,
}; MAX_SERVICES];

pub fn register_server(service_id: u32, pid: i32, core: u32) {
    if (service_id as usize) < MAX_SERVICES {
        unsafe {
            SERVERS[service_id as usize] = ServerReg {
                pid,
                core,
                used: true,
            };
        }
    }
}

pub fn wake_server(service_id: u32) {
    if service_id as usize >= MAX_SERVICES {
        return;
    }
    unsafe {
        let s = SERVERS[service_id as usize];
        if !s.used {
            return;
        }
        let t = scheduler::find_task_by_pid(s.pid);
        if !t.is_null() {
            wake(Some(t));
            scheduler::set_wake_hint(s.core as usize, t);
        }
        crate::lapic::send_ipi(crate::cpu::apic_id_of(s.core), 64);
    }
}

pub fn inbox_has_req(service_id: u32) -> bool {
    if service_id as usize >= MAX_SERVICES {
        return false;
    }
    unsafe {
        let base = crate::vmm::phys_to_virt(
            ARENA_PHYS
                + INBOX_OFF as u64
                + service_id as u64 * core::mem::size_of::<ServiceInbox>() as u64,
        );
        let ib = &*(base as *const ServiceInbox);
        let n = core::cmp::min(ib.count as usize, INBOX_CAP);
        for k in 0..n {
            let mi = ib.idx[k] as usize;
            if mi >= MAX_MAILBOXES {
                continue;
            }
            let mb = crate::vmm::phys_to_virt(ARENA_PHYS + MBOX_OFF as u64 + (mi * 64) as u64)
                as *const Mailbox;
            if core::ptr::read_volatile(&(*mb).status) == MBOX_REQ {
                return true;
            }
        }
    }
    false
}

pub fn inbox_add(service_id: u32, mbox_idx: u32) {
    if service_id as usize >= MAX_SERVICES {
        return;
    }
    unsafe {
        let base = crate::vmm::phys_to_virt(
            ARENA_PHYS
                + INBOX_OFF as u64
                + service_id as u64 * core::mem::size_of::<ServiceInbox>() as u64,
        );
        let count = &*(base as *const core::sync::atomic::AtomicU32);
        let slot = count.fetch_add(1, Ordering::Relaxed) as usize;
        if slot < INBOX_CAP {
            (*(base as *mut ServiceInbox)).idx[slot] = mbox_idx;
        }
    }
}

pub fn irq_ring_push(irq: usize, byte: u8) {
    unsafe {
        let off = IRQRING_OFF + irq * core::mem::size_of::<IrqRing>();
        let ring = &mut *(crate::vmm::phys_to_virt(ARENA_PHYS + off as u64) as *mut IrqRing);

        let head = core::ptr::read_volatile(&ring.head);
        let next = (head + 1) % IRQRING_CAP as u32;
        if next == core::ptr::read_volatile(&ring.tail) {
            return;
        }
        ring.data[head as usize] = byte;
        fence(Ordering::Release);
        core::ptr::write_volatile(&mut ring.head, next);
    }
}

#[repr(C, align(64))]
pub struct Mailbox {
    pub client_id: u32,
    pub service_id: u32,
    pub msg_offset: u32,
    pub status: u32,
    pub len: u32,
}

pub fn init() {
    unsafe {
        let phys = crate::pmm::alloc_pages(7) as u64; // order 4 = 16 contiguous pages
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

pub fn release_server_by_pid(pid: i32) {
    unsafe {
        for i in 0..MAX_SERVICES {
            if SERVERS[i].used && SERVERS[i].pid == pid {
                SERVERS[i] = ServerReg {
                    pid: -1,
                    core: 0,
                    used: false,
                };
            }
        }
    }
}
