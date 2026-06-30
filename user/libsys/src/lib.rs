#![no_std]

use core::sync::atomic::{AtomicU32, Ordering};

const ARENA: u64 = 0x5000_0000;
const MAX_MAILBOXES: usize = 64;
const MBOX_OFF: usize = 4096;
const BUFPOOL_OFF: usize = 0x10000;
const BUF_SIZE: usize = 4096;

pub const SVC_VFS: u32 = 1;
pub const SVC_ATA: u32 = 2;
pub const SVC_FS: u32 = 3;
pub const SVC_KBD: u32 = 4;

const EMPTY: u32 = 0;
const REQ: u32 = 1;
const REPLY: u32 = 2;

use core::panic::PanicInfo;
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    loop {}
}

#[derive(Copy, Clone)]
struct FD {
    conn: usize,
    handle: i32,
    used: bool,
}
const EMPTY_FD: FD = FD {
    conn: 0,
    handle: 0,
    used: false,
};
static mut FD_TABLE: [FD; 256] = [EMPTY_FD; 256];

pub const OP_BIND: u8 = 1;
pub const OP_RESOLVE: u8 = 2;
pub const OP_READ: u8 = 3;
pub const OP_WRITE: u8 = 4;
pub const OP_OPEN: u8 = 5;
pub const OP_IRQ: u8 = 6;

pub unsafe fn syscall(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let res: u64;
    let rcx: u64;
    let r11: u64;
    unsafe {
        core::arch::asm!("syscall", in("rax") syscall_num, in("rdi") arg1, in("rdx") arg2, in("r10") arg3, lateout("rax") res, out("rcx") rcx, out("r11") r11, options(nostack));
    }
    return res;
}

pub const IPC_MESSAGE_SIZE: usize = 4096;

#[repr(C, align(64))]
pub struct Mailbox {
    pub client_id: u32,
    pub service_id: u32,
    pub msg_offset: u32,
    pub status: u32,
    pub len: u32,
}

fn alloc_next() -> &'static AtomicU32 {
    unsafe { &*(ARENA as *const AtomicU32) }
}

pub fn mailboxes() -> &'static mut [Mailbox] {
    unsafe {
        core::slice::from_raw_parts_mut((ARENA + MBOX_OFF as u64) as *mut Mailbox, MAX_MAILBOXES)
    }
}

fn arena_buf(off: u32) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut((ARENA + off as u64) as *mut u8, BUF_SIZE) }
}

pub fn mbox_connect(service_id: u32) -> usize {
    let idx = alloc_next().fetch_add(1, Ordering::Relaxed) as usize;
    let mb = &mut mailboxes()[idx];
    mb.client_id = get_self_pid() as u32;
    mb.service_id = service_id;
    mb.msg_offset = (BUFPOOL_OFF + idx * BUF_SIZE) as u32;
    mb.status = EMPTY;
    idx
}

pub fn mbox_call(idx: usize, data: &[u8], out: &mut [u8]) -> usize {
    let mb = &mut mailboxes()[idx];
    let buf = arena_buf(mb.msg_offset);
    buf[..data.len()].copy_from_slice(data);
    mb.len = data.len() as u32;
    core::sync::atomic::fence(Ordering::Release);
    mb.status = REQ;

    while unsafe { core::ptr::read_volatile(&mb.status) } != REPLY {
        unsafe {
            syscall(9, 0, 0, 0, 0);
        }
    }
    core::sync::atomic::fence(Ordering::Acquire);
    let n = mb.len as usize;
    let buf = arena_buf(mb.msg_offset);
    out[..n].copy_from_slice(&buf[..n]);
    mb.status = EMPTY;
    n
}

type Handler = fn(req: &[u8], reply: &mut [u8]) -> usize;
static mut HANDLERS: [Option<Handler>; 256] = [None; 256];

pub fn register(op: u8, h: Handler) {
    unsafe {
        HANDLERS[op as usize] = Some(h);
    }
}

pub fn serve(my_service: u32) -> ! {
    loop {
        let mut found = false;
        for i in 0..MAX_MAILBOXES {
            let mb = &mut mailboxes()[i];
            if unsafe { core::ptr::read_volatile(&mb.status) } == REQ && mb.service_id == my_service
            {
                found = true;
                core::sync::atomic::fence(Ordering::Acquire);
                let n = mb.len as usize;
                let off = mb.msg_offset;
                let mut reply = [0u8; BUF_SIZE];
                let req_op = arena_buf(off)[0];
                let rn = unsafe {
                    if let Some(h) = HANDLERS[req_op as usize] {
                        h(&arena_buf(off)[..n], &mut reply)
                    } else {
                        0
                    }
                };
                let buf = arena_buf(off);
                buf[..rn].copy_from_slice(&reply[..rn]);
                mb.len = rn as u32;
                core::sync::atomic::fence(Ordering::Release);
                mb.status = REPLY;
            }
        }
        if !found {
            unsafe {
                syscall(9, 0, 0, 0, 0);
            }
        } // yield
    }
}

pub unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nostack, preserves_flags));
    }
    return v;
}

pub unsafe fn outb(port: u16, val: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("al") val, in("dx") port, options(nostack, preserves_flags));
    }
}

pub unsafe fn inw(port: u16) -> u16 {
    let v: u16;
    unsafe {
        core::arch::asm!("in ax, dx", in("dx") port, out("ax") v, options(nostack, preserves_flags));
    }
    return v;
}

pub fn print(s: &str) {
    unsafe {
        syscall(8, s.as_ptr() as u64, s.len() as u64, 0, 0);
    }
}

pub fn spawn(bytes: &[u8], cpu_id: u8) -> i32 {
    unsafe {
        return syscall(
            5,
            bytes.as_ptr() as u64,
            bytes.len() as u64,
            cpu_id as u64,
            0,
        ) as i32;
    }
}

pub fn vfs_resolve(path: &[u8]) -> u32 {
    let idx = mbox_connect(SVC_VFS);
    let mut req = [0u8; 256];
    req[0] = OP_RESOLVE;
    req[5..5 + path.len()].copy_from_slice(path);
    let mut out = [0u8; 4];
    mbox_call(idx, &req[..5 + path.len()], &mut out);
    u32::from_le_bytes([out[0], out[1], out[2], out[3]])
}
pub fn vfs_bind(path: &[u8], service_id: u32) -> i32 {
    let idx = mbox_connect(SVC_VFS);
    let mut req = [0u8; 256];
    req[0] = OP_BIND;
    req[1..5].copy_from_slice(&service_id.to_le_bytes());
    req[5..5 + path.len()].copy_from_slice(path);
    let mut out = [0u8; 4];
    mbox_call(idx, &req[..5 + path.len()], &mut out);
    i32::from_le_bytes([out[0], out[1], out[2], out[3]])
}

pub fn open(path: &[u8]) -> i32 {
    let sid = vfs_resolve(path);
    if sid == 0 {
        return -1;
    }
    let conn = mbox_connect(sid);
    let mut req = [0u8; 256];
    req[0] = OP_OPEN;
    req[1..1 + path.len()].copy_from_slice(path);
    let mut out = [0u8; 4];
    mbox_call(conn, &req[..1 + path.len()], &mut out);
    let handle = i32::from_le_bytes([out[0], out[1], out[2], out[3]]);
    if handle < 0 {
        return -1;
    }
    unsafe {
        for fd in 0..256 {
            if !FD_TABLE[fd].used {
                FD_TABLE[fd] = FD {
                    conn,
                    handle,
                    used: true,
                };
                return fd as i32;
            }
        }
    }
    -1
}
pub fn read(fd: i32, buf: &mut [u8]) -> i32 {
    unsafe {
        if fd < 0 || fd as usize >= 256 || !FD_TABLE[fd as usize].used {
            return -1;
        }
        let conn = FD_TABLE[fd as usize].conn;
        let mut req = [0u8; 9];
        req[0] = OP_READ;
        req[1..5].copy_from_slice(&FD_TABLE[fd as usize].handle.to_le_bytes());
        req[5..9].copy_from_slice(&(buf.len() as u32).to_le_bytes());
        mbox_call(conn, &req, buf) as i32
    }
}

pub fn write(fd: i32, buf: &[u8]) -> i32 {
    unsafe {
        if fd < 0 || fd as usize >= 256 || !FD_TABLE[fd as usize].used {
            return -1;
        }
        let conn = FD_TABLE[fd as usize].conn;
        let mut req = [0u8; 256];
        req[0] = OP_WRITE;
        let n = core::cmp::min(buf.len(), 255);
        req[1..1 + n].copy_from_slice(&buf[..n]);
        let mut out = [0u8; 4];
        mbox_call(conn, &req[..1 + n], &mut out);
        i32::from_le_bytes([out[0], out[1], out[2], out[3]])
    }
}

pub fn get_self_pid() -> i32 {
    unsafe {
        return syscall(15, 0, 0, 0, 0) as i32;
    }
}
