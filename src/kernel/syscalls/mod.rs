use crate::{
    cpu, elf,
    msr::{rdmsr, wrmsr},
    pmm, scheduler,
    serial::{self, write_hex},
    vmm,
};

const USER_STACK: u64 = 0x10000000;

unsafe extern "C" {
    fn syscall_entry();
}

pub unsafe fn init() {
    unsafe {
        // efer
        let efer_num = 0xC0000080u32;
        let efer = rdmsr(efer_num);
        wrmsr(efer_num, efer | 1);

        // star
        wrmsr(0xC0000081u32, (0x0008u64 << 32) | (0x0010u64 << 48));

        // lstar
        wrmsr(0xC0000082u32, syscall_entry as u64);

        // sfmask
        wrmsr(0xC0000084u32, 1u64 << 9);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(nr: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    match nr {
        0 => {
            // exit
            unsafe {
                let curr_task = *(cpu::get_current_task());
                core::arch::asm!("mov cr3, {}", in(reg) cpu::get_kernel_cr3());
                vmm::free_table(curr_task.cr3, 4);
                if !curr_task.stack.is_null() {
                    pmm::free_pages(0, curr_task.stack as u64);
                }
                core::arch::asm!("swapgs");
                scheduler::kill_current_task();
            }
            return 0;
        }
        1 => {
            let addr = pmm::alloc_pages(arg1 as usize) as u64;
            return addr;
        }
        2 => {
            pmm::free_pages(arg2 as usize, arg1);
            return 0;
        }
        3 => unsafe {
            let task = *(cpu::get_current_task());
            vmm::map_page(task.cr3 as *mut u64, arg1, arg2, arg3);
            return 0;
        },
        4 => unsafe {
            let task = *(cpu::get_current_task());
            vmm::unmap_page(task.cr3 as *mut u64, arg1);
            return 0;
        },
        5 => unsafe {
            let pml4 = vmm::create_address_space();
            let entry = elf::load(arg1 as *const u8, arg2 as usize, pml4);
            if entry == None {
                vmm::free_table(pml4 as u64, 4);
                return u64::MAX;
            }
            let stack_phys = pmm::alloc_pages(0) as u64;
            vmm::map_page(pml4, USER_STACK, stack_phys, 0x07);
            scheduler::spawn_user_task(entry.unwrap(), USER_STACK + 0x1000, pml4 as u64, 1);
            return 0;
        },
        _ => u64::MAX,
    }
}
