#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    loop {}
}

unsafe fn syscall(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let res: u64;
    let rcx: u64;
    let r11: u64;
    unsafe {
        core::arch::asm!("syscall", in("rax") syscall_num, in("rdi") arg1, in("rdx") arg2, in("r10") arg3, lateout("rax") res, out("rcx") rcx, out("r11") r11, options(nostack));
    }
    return res;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    unsafe {
        syscall(1, 0, 0, 0);

        syscall(0, 0, 0, 0);
    }

    loop {}
}
