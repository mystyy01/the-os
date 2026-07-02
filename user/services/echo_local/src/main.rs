#![no_std]
#![no_main]

use libsys::{OP_ECHO, OP_ECHO_TS, SVC_ECHO_LOCAL, rdtsc, register, serve};

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

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    register(OP_ECHO, on_echo);
    register(OP_ECHO_TS, on_echo_ts);
    serve(SVC_ECHO_LOCAL);
}
