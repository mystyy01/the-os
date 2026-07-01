use crate::vmm::phys_to_virt;

fn sig_match(p: *const u8, s: &[u8]) -> bool {
    unsafe {
        for i in 0..s.len() {
            if *p.add(i) != s[i] {
                return false;
            }
        }
    }
    true
}

unsafe fn find_madt(rsdp: *const u8) -> *const u8 {
    unsafe {
        let rev = *rsdp.add(15);
        if rev >= 2 {
            let xsdt_phys = core::ptr::read_unaligned(rsdp.add(24) as *const u64);
            let xsdt = phys_to_virt(xsdt_phys) as *const u8;
            let len = core::ptr::read_unaligned(xsdt.add(4) as *const u32) as usize;
            let n = (len - 36) / 8;
            for i in 0..n {
                let entry = core::ptr::read_unaligned(xsdt.add(36 + i * 8) as *const u64);
                let table = phys_to_virt(entry) as *const u8;
                if sig_match(table, b"APIC") {
                    return table;
                }
            }
        } else {
            let rsdt_phys = core::ptr::read_unaligned(rsdp.add(16) as *const u32);
            let rsdt = phys_to_virt(rsdt_phys as u64) as *const u8;
            let len = core::ptr::read_unaligned(rsdt.add(4) as *const u32) as usize;
            let n = (len - 36) / 4;
            for i in 0..n {
                let entry = core::ptr::read_unaligned(rsdt.add(36 + i * 4) as *const u32);
                let table = phys_to_virt(entry as u64) as *const u8;
                if sig_match(table, b"APIC") {
                    return table;
                }
            }
        }
        core::ptr::null()
    }
}

pub fn lapic_ids(multiboot2_info: *const u8, out: &mut [u8; 8]) -> usize {
    unsafe {
        let mut rsdp = core::ptr::null::<u8>();
        let mut tag_ptr = multiboot2_info.add(8);
        loop {
            let tag_type = *(tag_ptr as *const u32);
            let tag_size = *(tag_ptr.add(4) as *const u32);
            if tag_type == 0 {
                break;
            }
            if tag_type == 14 || tag_type == 15 {
                rsdp = tag_ptr.add(8);
            }
            tag_ptr = tag_ptr.add(((tag_size + 7) & !7) as usize);
        }
        if rsdp.is_null() {
            return 0;
        }

        let madt = find_madt(rsdp);
        if madt.is_null() {
            return 0;
        }

        let len = core::ptr::read_unaligned(madt.add(4) as *const u32) as usize;
        let mut off = 44;
        let mut count = 0;
        while off + 1 < len && count < out.len() {
            let entry_type = *madt.add(off);
            let entry_len = *madt.add(off + 1) as usize;
            if entry_len == 0 {
                break;
            }
            if entry_type == 0 {
                let apic_id = *madt.add(off + 3);
                let flags = core::ptr::read_unaligned(madt.add(off + 4) as *const u32);
                if flags & 1 != 0 {
                    out[count] = apic_id;
                    count += 1;
                }
            }
            off += entry_len;
        }
        count
    }
}
