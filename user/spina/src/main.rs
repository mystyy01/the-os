#![no_std]
#![no_main]

use libsys::syscall;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    loop {
        syscall(8, "A".as_ptr() as u64, 1, 0, 0);
    }
}
