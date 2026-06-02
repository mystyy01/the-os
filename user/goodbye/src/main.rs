#![no_std]
#![no_main]

use core::ptr::null;

use libsys::{IPCMessage, syscall};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    unsafe {
        syscall(1, 0, 0, 0);

        let mut msg = IPCMessage {
            sender_pid: -1,
            data: [0; 256],
            len: 0,
        };
        syscall(7, &mut msg as *mut _ as u64, 0, 0);
        syscall(8, msg.data.as_ptr() as u64, msg.len as u64, 0); // .data + .len, not .msg

        syscall(0, 0, 0, 0);
    }

    loop {}
}
