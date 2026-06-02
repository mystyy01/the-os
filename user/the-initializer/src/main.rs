#![no_std]
#![no_main]

use libsys::syscall;

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    unsafe {
        let kbd = include_bytes!("../../dist/kb_driver.elf");
        syscall(5, kbd.as_ptr() as u64, kbd.len() as u64, 0);

        loop {}
    }
}
