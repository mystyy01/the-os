#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use crate::{heap::KernelHeap, scheduler::spawn_task};

extern crate alloc;

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    loop {}
}

mod heap;
mod idt;
mod io;
mod pic;
mod pit;
mod pmm;
mod scheduler;
mod serial;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

fn write_vga(ptr: *mut u16, s: &str) -> () {
    unsafe {
        for (i, c) in s.bytes().enumerate() {
            *ptr.add(i) = (0x0f << 8) | c as u16;
        }
    }
}

fn test_task_a() {
    serial::write_str("Task a!\n");
    loop {}
}
fn test_task_b() {
    serial::write_str("Task b!\n");
    loop {}
}

#[unsafe(no_mangle)]
extern "C" fn kernel_main(multiboot2_info: *const u8) -> ! {
    idt::init();
    pic::init();
    pit::init(100);
    serial::init();
    serial::write_str("Hello world mr serial!\n");
    pmm::init(multiboot2_info);
    let ptr = 0xb8000 as *mut u16;
    write_vga(ptr, "hello world!");
    write_vga(ptr, "making the vector..");
    use alloc::vec::Vec;
    let mut v: Vec<u32> = Vec::new();
    v.push(42);
    write_vga(ptr, "we have made the vector!");
    unsafe { core::arch::asm!("sti") }
    spawn_task(test_task_a, 1);
    spawn_task(test_task_b, 1);
    unsafe {
        scheduler::start();
    }
    loop {}
}
