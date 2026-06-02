#![no_std]
#![no_main]

use libsys::{IPCMessage, inb, syscall};

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    // register for keyboard shit
    unsafe {
        syscall(10, 1, 0, 0);
    }
    loop {
        let mut msg = IPCMessage {
            sender_pid: -1,
            data: [0; 256],
            len: 0,
        };
        unsafe {
            syscall(7, &mut msg as *mut _ as u64, 0, 0);
        }
        unsafe {
            let sc = inb(0x60);
        }
        unsafe {
            syscall(8, "K".as_ptr() as u64, 1, 0);
        }
    }
}
