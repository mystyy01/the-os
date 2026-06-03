#![no_std]
#![no_main]

use libsys::{IPCMessage, syscall};

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    unsafe {
        let kbd = include_bytes!("../../dist/kb_driver.elf");
        let kbd_pid = syscall(5, kbd.as_ptr() as u64, kbd.len() as u64, 0, 0);
        let kbd_ipcd = syscall(13, kbd_pid, 0, 0, 0);
        let mut msg: IPCMessage = IPCMessage {
            data: [0; 256],
            len: 0,
        };
        syscall(
            11,
            kbd_ipcd,
            "hello".as_ptr() as u64,
            5,
            &mut msg as *mut _ as u64,
        );

        loop {}
    }
}
