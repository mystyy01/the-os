#![no_std]

use core::panic::PanicInfo;
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    loop {}
}

pub unsafe fn syscall(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let res: u64;
    let rcx: u64;
    let r11: u64;
    unsafe {
        core::arch::asm!("syscall", in("rax") syscall_num, in("rdi") arg1, in("rdx") arg2, in("r10") arg3, lateout("rax") res, out("rcx") rcx, out("r11") r11, options(nostack));
    }
    return res;
}

pub unsafe fn write(fd: u64, buf: *const u8, len: usize) -> u64 {
    unsafe {
        let res = syscall(6, fd, buf as u64, len as u64);
        return res;
    }
}

pub unsafe fn print(msg: &str) -> u64 {
    unsafe { write(1, msg.as_ptr(), msg.len()) }
}
