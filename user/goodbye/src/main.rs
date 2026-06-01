#![no_std]
#![no_main]

use libsys::syscall;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    unsafe {
        syscall(1, 0, 0, 0);

        syscall(0, 0, 0, 0);
    }

    loop {}
}
