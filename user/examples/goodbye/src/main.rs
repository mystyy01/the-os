#![no_std]
#![no_main]

use libsys::{IPCMessage, syscall};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    unsafe {
        syscall(1, 0, 0, 0, 0);

        let mut msg = IPCMessage {
            data: [0; 256],
            len: 0,
        };
        let hello_ipcd = syscall(13, 1, 0, 0, 0);
        syscall(7, &mut msg as *mut _ as u64, hello_ipcd, 0, 0);
        syscall(8, msg.data.as_ptr() as u64, msg.len as u64, 0, 0);

        syscall(0, 0, 0, 0, 0);
    }

    loop {}
}
