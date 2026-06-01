use crate::pmm::{alloc_pages, free_pages};

pub const HHDM_BASE: u64 = 0xFFFF800000000000;

pub fn phys_to_virt(phys: u64) -> u64 {
    return phys + HHDM_BASE;
}

unsafe fn get_or_create(table: *mut u64, idx: u64) -> *mut u64 {
    unsafe {
        let entry = table.add(idx as usize);
        if *entry & 1 == 0 {
            let new_table = crate::pmm::alloc_pages(0);
            core::ptr::write_bytes(phys_to_virt(new_table as u64) as *mut u8, 0, 4096);
            *entry = new_table as u64 | 0x07;
        }
        return (phys_to_virt(*entry & !0xFFF)) as *mut u64;
    }
}

unsafe extern "C" {
    static mut PML4: u64;
}

pub unsafe fn create_address_space() -> *mut u64 {
    unsafe {
        let pml4 = crate::pmm::alloc_pages(0);
        let pml4_virt = phys_to_virt(pml4 as u64) as *mut u64;
        core::ptr::write_bytes(pml4_virt as *mut u8, 0, 4096);

        let boot_pml4 = phys_to_virt(&raw const PML4 as u64) as *mut u64;
        *pml4_virt.add(256) = *boot_pml4.add(256);
        *pml4_virt.add(511) = *boot_pml4.add(511);

        return pml4 as *mut u64;
    }
}

pub unsafe fn map_page(pml4: *mut u64, virt: u64, phys: u64, flags: u64) {
    let pml4_idx = (virt >> 39) & 0x1FF;
    let pdpt_idx = (virt >> 30) & 0x1FF;
    let pd_idx = (virt >> 21) & 0x1FF;
    let pt_idx = (virt >> 12) & 0x1FF;

    unsafe {
        let pml4 = phys_to_virt(pml4 as u64) as *mut u64;
        let mut entry = *pml4.add(pml4_idx as usize);
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
        let pdpt = phys_to_virt(entry & !0xFFF) as *mut u64;
        entry = *pdpt.add(pdpt_idx as usize);
        if entry & 1 == 0 {
            return;
        }
        let pd = phys_to_virt(entry & !0xFFF) as *mut u64;
        entry = *pd.add(pd_idx as usize);
        if entry & 1 == 0 {
            return;
        }
        let pt = phys_to_virt(entry & !0xFFF) as *mut u64;
        *pt.add(pt_idx as usize) = 0;
        core::arch::asm!("invlpg [{}]", in(reg) virt, options(nostack));
    }
}

pub unsafe fn free_table(table: u64, depth: u8) {
    unsafe {
        let virt = phys_to_virt(table) as *mut u64;
        for i in 0..512 {
            if depth == 4 && (i == 0 || i == 256 || i == 511) {
                continue;
            }
            let entry = *virt.add(i);
            if entry & 1 == 0 {
                continue;
            }
            if depth == 1 {
                free_pages(0, entry & !0xFFF);
            } else {
                free_table(entry & !0xFFF, depth - 1);
            }
        }
        free_pages(0, table);
    }
}
