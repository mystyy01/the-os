use crate::pmm::{alloc_pages, free_pages};

unsafe fn get_or_create(table: *mut u64, idx: u64) -> *mut u64 {
    let entry = table.add(idx as usize);
    if *entry & 1 == 0 {
        let new_table = crate::pmm::alloc_pages(0);
        core::ptr::write_bytes(new_table, 0, 4096);
        *entry = new_table as u64 | 0x07;
    }
    return ((*entry & !0xFFF) as *mut u64);
}

unsafe extern "C" {
    static mut PML4: u64;
}

pub unsafe fn create_address_space() -> *mut u64 {
    unsafe {
        let pml4 = crate::pmm::alloc_pages(0) as *mut u64;
        core::ptr::write_bytes(pml4, 0, 4096);

        let boot_pml4 = &raw mut PML4 as *mut u64;
        *pml4 = *boot_pml4;

        return pml4;
    }
}

pub unsafe fn switch_address_space(pml4: *mut u64) {
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) pml4 as u64, options(nostack));
    }
}

pub unsafe fn map_page(pml4: *mut u64, virt: u64, phys: u64, flags: u64) {
    let pml4_idx = (virt >> 39) & 0x1FF;
    let pdpt_idx = (virt >> 30) & 0x1FF;
    let pd_idx = (virt >> 21) & 0x1FF;
    let pt_idx = (virt >> 12) & 0x1FF;

    unsafe {
        let pdpt = get_or_create(pml4, pml4_idx);
        let pd = get_or_create(pdpt, pdpt_idx);
        let pt = get_or_create(pd, pd_idx);
        *pt.add(pt_idx as usize) = phys | flags;
    }
}

pub unsafe fn unmap_page(pml4: *mut u64, virt: u64) {
    let pml4_idx = (virt >> 39) & 0x1FF;
    let pdpt_idx = (virt >> 30) & 0x1FF;
    let pd_idx = (virt >> 21) & 0x1FF;
    let pt_idx = (virt >> 12) & 0x1FF;

    unsafe {
        let mut entry = *pml4.add(pml4_idx as usize);
        if entry & 1 == 0 {
            return;
        }
        let pdpt = (entry & !0xFFF) as *mut u64;
        entry = *pdpt.add(pdpt_idx as usize);
        if entry & 1 == 0 {
            return;
        }
        let pd = (entry & !0xFFF) as *mut u64;
        entry = *pd.add(pd_idx as usize);
        if entry & 1 == 0 {
            return;
        }
        let pt = (entry & !0xFFF) as *mut u64;
        *pt.add(pt_idx as usize) = 0;
        core::arch::asm!("invlpg [{}]", in(reg) virt, options(nostack));
    }
}

pub unsafe fn free_table(table: *mut u64, depth: u8) {
    unsafe {
        for i in 0..512 {
            if depth == 4 && i == 0 {
                continue;
            }
            let entry = table.add(i);
            if *entry & 1 == 0 {
                continue;
            }
            if depth == 1 {
                free_pages(0, (*entry & !0xFFF) as u64);
            } else {
                let next_table = (*entry & !0xFFF) as *mut u64;
                free_table(next_table, depth - 1);
            }
        }
        free_pages(0, table as u64);
    }
}

pub unsafe fn jump_to_userspace(entry: u64, user_stack: u64) {
    unsafe {
        core::arch::asm!(
            "push 0x1b",
            "push {stack}",
            "push 0x202",
            "push 0x23",
            "push {entry}",
            "iretq",
            entry = in(reg) entry,
            stack = in(reg) user_stack,
            options(noreturn)
        );
    }
}
