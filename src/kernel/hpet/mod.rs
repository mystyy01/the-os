use crate::vmm::phys_to_virt;
use core::sync::atomic::{AtomicU64, Ordering};

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

unsafe fn find_table(rsdp: *const u8, sig: &[u8]) -> *const u8 {
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
                if sig_match(table, sig) {
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
                if sig_match(table, sig) {
                    return table;
                }
            }
        }
        core::ptr::null()
    }
}

pub fn find_hpet(multiboot2_info: *const u8) -> *const u8 {
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
            return core::ptr::null();
        }
        find_table(rsdp, b"HPET")
    }
}

static HPET_BASE: AtomicU64 = AtomicU64::new(0);
static HPET_PERIOD_FS: AtomicU64 = AtomicU64::new(0);

pub fn init(multiboot2_info: *const u8) -> bool {
    let table = find_hpet(multiboot2_info);
    if table.is_null() {
        return false;
    }
    let mmio_phys = unsafe { core::ptr::read_unaligned(table.add(44) as *const u64) };
    let base = phys_to_virt(mmio_phys);
    let caps = unsafe { core::ptr::read_volatile(base as *const u64) };
    let period_fs = caps >> 32;

    HPET_BASE.store(base, Ordering::SeqCst);
    HPET_PERIOD_FS.store(period_fs, Ordering::SeqCst);

    unsafe {
        core::ptr::write_volatile((base as *mut u64).byte_add(0x10), 1);
    }
    true
}

pub fn now_ns() -> u64 {
    let base = HPET_BASE.load(Ordering::SeqCst);
    let period_fs = HPET_PERIOD_FS.load(Ordering::SeqCst);
    let ticks = unsafe { core::ptr::read_volatile((base as *const u64).byte_add(0xF0)) };
    ticks.wrapping_mul(period_fs) / 1_000_000
}

pub fn sleep_ns(ns: u64) {
    let start = now_ns();
    while now_ns() - start < ns {
        crate::scheduler::yield_now();
    }
}
