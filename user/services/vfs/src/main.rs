#![no_std]
#![no_main]

use libsys::{IPCMessage, syscall};

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    loop {}
}
