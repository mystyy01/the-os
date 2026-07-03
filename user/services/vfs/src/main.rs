#![no_std]
#![no_main]

use libsys::{OP_BIND, OP_RESOLVE, SVC_VFS, register, serve};

const MAX_VFS_BINDINGS: usize = 32;
const MAX_PATH_LEN: usize = 64;

#[derive(Copy, Clone)]
struct Binding {
    path: [u8; MAX_PATH_LEN],
    path_len: usize,
    endpoint: u32,
    used: bool,
}

const EMPTY_BINDING: Binding = Binding {
    path: [0u8; MAX_PATH_LEN],
    path_len: 0,
    endpoint: 0,
    used: false,
};

static mut VFS_BINDINGS: [Binding; MAX_VFS_BINDINGS] = [EMPTY_BINDING; MAX_VFS_BINDINGS];

fn bind(sid: u32, path: &[u8]) -> i32 {
    unsafe {
        for idx in 0..MAX_VFS_BINDINGS {
            let mut b = VFS_BINDINGS[idx];
            if !b.used {
                for (i, byte) in path.iter().enumerate() {
                    b.path[i] = *byte;
                }
                b.path_len = path.len();
                b.endpoint = sid;
                b.used = true;
                VFS_BINDINGS[idx] = b;
                return 0;
            }
        }
    }
    -1
}
fn resolve(path: &[u8]) -> u32 {
    let mut best: u32 = 0;
    let mut best_len: usize = 0;
    unsafe {
        for binding in VFS_BINDINGS {
            if !binding.used {
                continue;
            }
            let bpath = &binding.path[..binding.path_len];
            if path.len() >= bpath.len()
                && &path[..bpath.len()] == bpath
                && binding.path_len >= best_len
            {
                best = binding.endpoint;
                best_len = binding.path_len;
            }
        }
    }
    best
}

fn on_bind(req: &[u8], reply: &mut [u8]) -> usize {
    let mut r: i32 = -1;
    if req.len() >= 5 {
        let sid = u32::from_le_bytes([req[1], req[2], req[3], req[4]]);
        r = bind(sid, &req[5..]);
    }
    reply[..4].copy_from_slice(&r.to_le_bytes());
    4
}
fn on_resolve(req: &[u8], reply: &mut [u8]) -> usize {
    let mut r: u32 = 0;
    if req.len() >= 5 {
        r = resolve(&req[5..]);
    }
    reply[..4].copy_from_slice(&r.to_le_bytes());
    4
}
#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    register(OP_BIND, on_bind);
    register(OP_RESOLVE, on_resolve);
    serve(SVC_VFS);
    loop {}
}
