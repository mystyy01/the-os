#![no_std]

use core::panic::PanicInfo;
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    loop {}
}

pub unsafe fn syscall(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let res: u64;
    let rcx: u64;
    let r11: u64;
    unsafe {
        core::arch::asm!("syscall", in("rax") syscall_num, in("rdi") arg1, in("rdx") arg2, in("r10") arg3, lateout("rax") res, out("rcx") rcx, out("r11") r11, options(nostack));
    }
    return res;
}

#[derive(Clone, Copy)]
pub struct IPCMessage {
    pub data: [u8; 256],
    pub len: usize,
}

pub unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nostack, nomem));
    return v;
}
