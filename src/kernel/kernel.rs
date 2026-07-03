#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use core::sync::atomic::{AtomicU32, Ordering};

use crate::{
    heap::KernelHeap,
    idle::setup_idle,
    pmm::{PAGE_SIZE, alloc_pages},
    scheduler::{spawn_task, yield_now},
    vmm::phys_to_virt,
};

extern crate alloc;

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    loop {}
}

static mut KSP_A: u64 = 0;
static mut KSP_B: u64 = 0;

static AP_READY: AtomicU32 = AtomicU32::new(0);

mod acpi;
mod cpu;
mod elf;
mod fb;
mod gdt;
mod heap;
mod hpet;
mod idle;
mod idt;
mod io;
mod ipc;
mod irq;
mod lapic;
mod msr;
mod pci;
mod pic;
mod pit;
mod pmm;
mod scheduler;
mod serial;
mod syscalls;
mod vmm;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    serial::write_str_raw("PANIC\n");
    if let Some(loc) = _info.location() {
        serial::write_str_raw(loc.file());
        serial::write_str_raw("\n");
        serial::write_hex_raw(loc.line() as u64);
        serial::write_str_raw("\n");
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
static mut SLEEPER: *mut scheduler::Task = core::ptr::null_mut();
fn test_task_a() {
    serial::write_str("A blocking\n");
    unsafe {
        SLEEPER = cpu::get_current_task();
        scheduler::block_current();
    }
    serial::write_str("A woke!\n");
    loop {
        unsafe {
            scheduler::yield_now();
        }
    }
}

unsafe extern "C" {
    static stack_top: u8;
}

extern "C" fn ap_main() -> ! {
    let ap_stack_top: u64;
    unsafe { core::arch::asm!("mov {}, rsp", out(reg) ap_stack_top) };

    AP_READY.store(1, Ordering::SeqCst);

    serial::write_str("AP: lapic init\n");
    lapic::init();
    lapic::init_timer();
    serial::write_str("AP: idt init\n");
    idt::init();
    serial::write_str("AP: cpu init\n");
    let id = unsafe { cpu::index_of(lapic::id()) };
    unsafe {
        cpu::init(id, ap_stack_top);
        serial::write_str("AP: gdt init\n");
        syscalls::init();
        gdt::init(id, ap_stack_top);
        serial::write_str("AP: sti\n");
        core::arch::asm!("sti");

        setup_idle();

        serial::write_str("AP: start\n");
        scheduler::start();
    }
    loop {}
}

#[unsafe(no_mangle)]
extern "C" fn kernel_main(multiboot2_info: *const u8) -> ! {
    serial::init();
    for device in 0..32u8 {
        let (vendor, dev) = pci::vendor_device(0, device, 0);
        if vendor != 0xFFFF {
            serial::write_str("pci dev ");
            serial::write_hex(device as u64);
            serial::write_str(" vendor ");
            serial::write_hex(vendor as u64);
            serial::write_str(" device ");
            serial::write_hex(dev as u64);
            serial::write_str("\n");
        }
    }
    fb::init(multiboot2_info);
    serial::write_str("init idt\n");
    idt::init();
    serial::write_str("init gdt\n");
    gdt::init(0, &raw const stack_top as u64);

    unsafe {
        cpu::init(0, &raw const stack_top as u64);

        syscalls::init();
    }
    serial::write_str("init pmm\n");
    pmm::init(multiboot2_info);

    hpet::init(multiboot2_info);

    ipc::init();
    pic::init();
    lapic::init();
    lapic::init_timer();
    let tramp = include_bytes!("../../build/ap_trampoline.bin");
    unsafe {
        let dst = vmm::phys_to_virt(0x8000) as *mut u8;
        core::ptr::copy_nonoverlapping(tramp.as_ptr(), dst, tramp.len());

        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        *(vmm::phys_to_virt(0x8FF0) as *mut u32) = cr3 as u32;

        *(vmm::phys_to_virt(0x9000) as *mut u64) = ap_main as u64;

        let bsp = lapic::id();
        cpu::register_cpu(0, bsp);

        let mut madt_ids = [0u8; 8];
        let found = acpi::lapic_ids(multiboot2_info, &mut madt_ids);

        let mut candidates = [0u8; 8];
        let mut n = 0;
        if found == 0 {
            let mut a = 1u8;
            while a < 8 {
                candidates[n] = a;
                n += 1;
                a += 1;
            }
        } else {
            for pass in 0..2u8 {
                for k in 0..found {
                    let id = madt_ids[k];
                    if id == bsp {
                        continue;
                    }
                    if id % 2 == pass && n < 8 {
                        candidates[n] = id;
                        n += 1;
                    }
                }
            }
        }

        let mut seq = 1u32;
        let mut i = 0;
        while i < n && seq < 8 {
            let apic = candidates[i];
            i += 1;

            AP_READY.store(0, Ordering::SeqCst);

            let ap_stack = pmm::alloc_pages(2) as u64 + 4 * 4096;
            *(vmm::phys_to_virt(0x8FF8) as *mut u64) = vmm::phys_to_virt(ap_stack);

            cpu::register_cpu(seq, apic);

            lapic::send_init(apic);
            let deadline = hpet::now_ns() + 10_000;
            while hpet::now_ns() < deadline {
                core::hint::spin_loop();
            }
            lapic::send_sipi(apic, 0x08);
            let deadline = hpet::now_ns() + 200_000;
            while hpet::now_ns() < deadline {
                core::hint::spin_loop();
            }
            lapic::send_sipi(apic, 0x08);

            let mut up = false;
            let deadline = hpet::now_ns() + 100_000_000;
            while hpet::now_ns() < deadline {
                if AP_READY.load(Ordering::SeqCst) != 0 {
                    up = true;
                    break;
                }
                core::hint::spin_loop();
            }
            if !up {
                serial::write_str("AP timeout\n");
            }
            seq += 1;
        }
    }
    unsafe { core::arch::asm!("sti") }
    serial::write_str("Entering user space\n");
    setup_idle();

    let bytes = include_bytes!("../../user/dist/the-initializer.elf");
    unsafe {
        let pml4 = vmm::create_address_space();
        let entry = elf::load(bytes.as_ptr(), bytes.len(), pml4);
        let user_stack: u64 = 0x10000000;
        let stack_pages: u64 = 512;
        let mut i: u64 = 0;
        while i < stack_pages {
            let phys = pmm::alloc_pages(0);
            vmm::map_page(pml4, user_stack + i * 0x1000, phys as u64, 0x07);
            i += 1;
        }
        scheduler::spawn_user_task(
            entry.unwrap(),
            user_stack + stack_pages * 0x1000,
            pml4 as u64,
            1,
            0,
        );
    }

    unsafe {
        scheduler::start();
    }
    loop {}
}
