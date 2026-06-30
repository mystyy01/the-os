#![no_std]
#![no_main]

use libsys::{
    IPCMessage, OP_BIND, OP_READ, OP_RESOLVE, OP_WRITE, call, connect, recv_any, register, reply,
    serve, syscall, vfs_resolve,
};
const MAX_VFS_BINDINGS: usize = 32;
const MAX_PATH_LEN: usize = 64;

#[derive(Copy, Clone)]
struct Binding {
    path: [u8; MAX_PATH_LEN],
    path_len: usize,
    endpoint: i32,
    used: bool,
}

const EMPTY_BINDING: Binding = Binding {
    path: [0u8; MAX_PATH_LEN],
    path_len: 0,
    endpoint: -1,
    used: false,
};

static mut VFS_BINDINGS: [Binding; MAX_VFS_BINDINGS] = [EMPTY_BINDING; MAX_VFS_BINDINGS];

fn bind(pid: i32, path: &[u8]) -> i32 {
    // make like this path = this pid
    // vfs is basically a big hashmap
    unsafe {
        for idx in 0..MAX_VFS_BINDINGS {
            let mut binding = VFS_BINDINGS[idx];
            if binding.used == false {
                for (i, byte) in path.iter().enumerate() {
                    binding.path[i] = *byte;
                }
                binding.path_len = path.len();
                binding.endpoint = pid;
                binding.used = true;
                VFS_BINDINGS[idx] = binding;
                return 0;
            }
        }
    }
    return -1;
}

fn resolve(path: &[u8]) -> i32 {
    let mut best: i32 = -1;
    let mut best_len: usize = 0;
    unsafe {
        for binding in VFS_BINDINGS {
            if !binding.used {
                continue;
            }
            let bpath = &binding.path[..binding.path_len];
            if path.len() >= bpath.len() && &path[..bpath.len()] == bpath {
                if binding.path_len >= best_len {
                    best = binding.endpoint;
                    best_len = binding.path_len;
                }
            }
        }
    }
    best
}

fn on_bind(req: &IPCMessage, reply: &mut IPCMessage) {
    let mut r: i32 = -1;
    if req.len >= 5 {
        let pid = i32::from_le_bytes([req.data[1], req.data[2], req.data[3], req.data[4]]);
        let path = &req.data[5..req.len];
        r = bind(pid, path);
    }
    reply.data[..4].copy_from_slice(&r.to_le_bytes());
    reply.len = 4;
}

fn on_resolve(req: &IPCMessage, reply: &mut IPCMessage) {
    let mut r: i32 = -1;
    if req.len >= 5 {
        let path = &req.data[5..req.len];
        r = resolve(path);
    }
    reply.data[..4].copy_from_slice(&r.to_le_bytes());
    reply.len = 4;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    register(OP_BIND, on_bind);
    register(OP_RESOLVE, on_resolve);
    serve();
}
