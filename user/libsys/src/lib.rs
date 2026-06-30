#![no_std]

const VFS_PID: i32 = 1;

use core::panic::PanicInfo;
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    loop {}
}

#[derive(Copy, Clone)]
struct FD {
    ipcd: i32,
    handle: i32,
    used: bool,
}
const EMPTY_FD: FD = FD {
    ipcd: -1,
    handle: 0,
    used: false,
};
static mut FD_TABLE: [FD; 256] = [EMPTY_FD; 256];

type Handler = fn(req: &IPCMessage, reply: &mut IPCMessage);
static mut HANDLERS: [Option<Handler>; 256] = [None; 256];

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

#[derive(Clone, Copy)]
pub struct IPCMessage {
    pub data: [u8; IPC_MESSAGE_SIZE],
    pub len: usize,
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

pub fn call(ipcd: i32, msg: IPCMessage) -> IPCMessage {
    let mut reply = IPCMessage {
        data: [0; IPC_MESSAGE_SIZE],
        len: 0,
    };
    unsafe {
        syscall(
            11,
            ipcd as u64,
            msg.data.as_ptr() as u64,
            msg.len as u64,
            &mut reply as *mut _ as u64,
        );
    }
    return reply;
}

pub fn reply(ipcd: i32, data: &[u8]) {
    unsafe {
        syscall(12, data.as_ptr() as u64, data.len() as u64, ipcd as u64, 0);
    }
}

pub fn connect(peer_pid: i32) -> i32 {
    unsafe {
        return syscall(13, peer_pid as u64, 0, 0, 0) as i32;
    }
}

pub fn recv_any() -> (i32, IPCMessage) {
    // recv any message from currently connected clients
    unsafe {
        let mut msg = IPCMessage {
            data: [0; IPC_MESSAGE_SIZE],
            len: 0,
        };
        let ipcd = syscall(14, &mut msg as *mut _ as u64, 0, 0, 0);

        return (ipcd as i32, msg);
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

pub fn vfs_resolve(path: &[u8]) -> i32 {
    let mut data = [0u8; IPC_MESSAGE_SIZE];
    data[0] = OP_RESOLVE;
    data[5..5 + path.len()].copy_from_slice(path);
    let msg = IPCMessage {
        data,
        len: 5 + path.len(),
    };

    let ipcd = connect(VFS_PID);
    let r = call(ipcd, msg);
    i32::from_le_bytes([r.data[0], r.data[1], r.data[2], r.data[3]])
}
pub fn vfs_bind(path: &[u8], endpoint: i32) -> i32 {
    let mut data = [0u8; IPC_MESSAGE_SIZE];
    data[0] = OP_BIND;
    data[1..5].copy_from_slice(&endpoint.to_le_bytes());
    data[5..5 + path.len()].copy_from_slice(path);
    let msg = IPCMessage {
        data,
        len: 5 + path.len(),
    };

    let ipcd = connect(VFS_PID);
    let r = call(ipcd, msg);
    i32::from_le_bytes([r.data[0], r.data[1], r.data[2], r.data[3]])
}

pub fn register(op: u8, h: Handler) {
    unsafe {
        HANDLERS[op as usize] = Some(h);
    }
}

pub fn serve() -> ! {
    loop {
        let (ipcd, msg) = recv_any();
        let mut reply_msg = IPCMessage {
            data: [0; IPC_MESSAGE_SIZE],
            len: 0,
        };
        unsafe {
            if let Some(h) = HANDLERS[msg.data[0] as usize] {
                h(&msg, &mut reply_msg);
            }
        }
        reply(ipcd, &reply_msg.data[..reply_msg.len]);
    }
}

pub fn open(path: &[u8]) -> i32 {
    let endpoint = vfs_resolve(path);
    if endpoint < 0 {
        return -1;
    }
    let ipcd = connect(endpoint);
    if ipcd < 0 {
        return -1;
    }
    let mut data = [0u8; IPC_MESSAGE_SIZE];
    data[0] = OP_OPEN;
    data[1..1 + path.len()].copy_from_slice(path);
    let r = call(
        ipcd,
        IPCMessage {
            data,
            len: 1 + path.len(),
        },
    );
    let handle = i32::from_le_bytes([r.data[0], r.data[1], r.data[2], r.data[3]]);
    if handle < 0 {
        return -1;
    }
    unsafe {
        for fd in 0..256 {
            if !FD_TABLE[fd].used {
                FD_TABLE[fd] = FD {
                    ipcd,
                    handle: handle,
                    used: true,
                };
                return fd as i32;
            }
        }
    }
    return -1;
}
pub fn read(fd: i32, buf: &mut [u8]) -> i32 {
    unsafe {
        if fd < 0 || fd as usize >= 256 || !FD_TABLE[fd as usize].used {
            return -1;
        }
        let ipcd = FD_TABLE[fd as usize].ipcd;

        let mut data = [0u8; IPC_MESSAGE_SIZE];
        data[0] = OP_READ;
        data[1..5].copy_from_slice(&FD_TABLE[fd as usize].handle.to_le_bytes());
        data[5..9].copy_from_slice(&(buf.len() as u32).to_le_bytes());
        let msg = IPCMessage { data, len: 9 };

        let r = call(ipcd, msg);
        let n = core::cmp::min(r.len, buf.len());
        buf[..n].copy_from_slice(&r.data[..n]);
        n as i32
    }
}

pub fn write(fd: i32, buf: &[u8]) -> i32 {
    unsafe {
        if fd < 0 || fd as usize >= 256 || !FD_TABLE[fd as usize].used {
            return -1;
        }
        let ipcd = FD_TABLE[fd as usize].ipcd;
        let mut data = [0u8; IPC_MESSAGE_SIZE];
        data[0] = OP_WRITE;
        let n = core::cmp::min(buf.len(), 255);
        data[1..1 + n].copy_from_slice(&buf[..n]);
        let msg = IPCMessage { data, len: 1 + n };
        let r = call(ipcd, msg);
        i32::from_le_bytes([r.data[0], r.data[1], r.data[2], r.data[3]])
    }
}

pub fn get_self_pid() -> i32 {
    unsafe {
        return syscall(15, 0, 0, 0, 0) as i32;
    }
}
