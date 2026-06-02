#![no_std]
#![no_main]

use libsys::syscall;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    unsafe {
        syscall(1, 0, 0, 0);

        let goodbye = include_bytes!("../../dist/goodbye.elf");

        syscall(5, goodbye.as_ptr() as u64, goodbye.len() as u64, 0);

        syscall(9, 0, 0, 0);

        syscall(6, 2, "hello".as_ptr() as u64, 5);

        syscall(0, 0, 0, 0);
    }

    loop {}
}
