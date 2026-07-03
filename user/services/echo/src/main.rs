#![no_std]
#![no_main]

use libsys::{OP_ECHO, OP_ECHO_TS, PP_BASE, PP_ITERS, PP_WARMUP, SVC_ECHO, rdtsc, register, serve};

fn on_echo(req: &[u8], reply: &mut [u8]) -> usize {
    let n = req.len();
    reply[..n].copy_from_slice(req);
    n
}

fn on_echo_ts(_req: &[u8], reply: &mut [u8]) -> usize {
    let t2 = rdtsc();
    let t3 = rdtsc();
    reply[0..8].copy_from_slice(&t2.to_le_bytes());
    reply[8..16].copy_from_slice(&t3.to_le_bytes());
    16
}

fn floor_server() {
    let word = PP_BASE as *mut u32;
    let srv = (PP_BASE + 8) as *mut u64;
    let mut expect = 1u32;
    for _ in 0..PP_WARMUP + PP_ITERS {
        while unsafe { core::ptr::read_volatile(word) } != expect {
            core::hint::spin_loop();
        }
        unsafe {
            core::ptr::write_volatile(srv, rdtsc());
            core::ptr::write_volatile(word, expect + 1);
        }
        expect += 2;
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    floor_server();
    register(OP_ECHO, on_echo);
    register(OP_ECHO_TS, on_echo_ts);
    serve(SVC_ECHO);

    loop {}
}
