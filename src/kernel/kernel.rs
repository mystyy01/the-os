#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use crate::{heap::KernelHeap, idle::setup_idle, pmm::alloc_pages, scheduler::spawn_task};

extern crate alloc;

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    loop {}
}

mod cpu;
mod elf;
mod gdt;
mod heap;
mod idle;
mod idt;
mod io;
mod msr;
mod pic;
mod pit;
mod pmm;
mod scheduler;
mod serial;
mod syscalls;
mod vmm;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    serial::write_str("PANIC\n");
    if let Some(loc) = _info.location() {
        serial::write_str(loc.file());
        serial::write_str("\n");
        serial::write_hex(loc.line() as u64);
        serial::write_str("\n");
    }
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

fn test_user_func() {
    unsafe {
        let addr: u64;
        core::arch::asm!("mov rax, 1", "mov rdi, 0", "syscall", out("rax") addr, options(nostack));
        core::arch::asm!(
            "syscall",
            in("rax") 2u64,
            in("rdi") addr,
            in("rdx") 0u64,
            lateout("rax") _,
            options(nostack)
        );
        core::arch::asm!("mov rax, 0", "syscall", options(nostack));
    }
    loop {}
}

unsafe extern "C" {
    static stack_top: u8;
}

#[unsafe(no_mangle)]
extern "C" fn kernel_main(multiboot2_info: *const u8) -> ! {
    serial::init();
    serial::write_str("init idt\n");
    idt::init();
    serial::write_str("init gdt\n");
    gdt::init(&raw const stack_top as u64);
    unsafe {
        cpu::init(0, &raw const stack_top as u64);

        syscalls::init();
    }
    serial::write_str("init pmm\n");
    pmm::init(multiboot2_info);
    pic::init();
    pit::init(100);
    unsafe { core::arch::asm!("sti") }
    serial::write_str("Entering user space\n");
    setup_idle();
    // spawn_task(test_task_a, 1);
    // spawn_task(test_task_b, 1);

    let bytes = include_bytes!("../../user/dist/hello.elf");

    unsafe {
        let pml4 = vmm::create_address_space();
        let entry = elf::load(bytes.as_ptr(), bytes.len(), pml4);
        let phys = pmm::alloc_pages(0);
        let user_stack: u64 = 0x10000000;
        vmm::map_page(pml4, user_stack, phys as u64, 0x07);

        scheduler::spawn_user_task(entry.unwrap(), user_stack + 0x1000, pml4 as u64, 1);
    }

    let ptr = 0xb8000 as *mut u16;
    write_vga(ptr, "hello world!");
    write_vga(ptr, "making the vector..");
    use alloc::vec::Vec;
    let mut v: Vec<u32> = Vec::new();
    v.push(42);
    write_vga(ptr, "we have made the vector!");
    unsafe { core::arch::asm!("sti") }

    unsafe {
        scheduler::start();
    }
    loop {}
}
