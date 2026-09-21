use crate::pmm::PAGE_SIZE;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub const MAX_MODULES: usize = 4;

static MOD_START: [AtomicU64; MAX_MODULES] = [const { AtomicU64::new(0) }; MAX_MODULES];
static MOD_END: [AtomicU64; MAX_MODULES] = [const { AtomicU64::new(0) }; MAX_MODULES];
static MOD_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn init(multiboot2_info: *const u8) {
    unsafe {
        let mut tag_ptr = multiboot2_info.add(8);
        loop {
            let tag_type = *(tag_ptr as *const u32);
            let tag_size = *(tag_ptr.add(4) as *const u32);
            if tag_type == 0 {
                break;
            }
            if tag_type == 3 {
                let n = MOD_COUNT.load(Ordering::SeqCst);
                if n < MAX_MODULES {
                    let start = *(tag_ptr.add(8) as *const u32) as u64;
                    let end = *(tag_ptr.add(12) as *const u32) as u64;
                    if end > start {
                        MOD_START[n].store(start, Ordering::SeqCst);
                        MOD_END[n].store(end, Ordering::SeqCst);
                        MOD_COUNT.store(n + 1, Ordering::SeqCst);
                    }
                }
            }
            tag_ptr = tag_ptr.add(((tag_size + 7) & !7) as usize);
        }
    }
}

pub fn count() -> usize {
    MOD_COUNT.load(Ordering::SeqCst)
}

pub fn get(idx: usize) -> Option<(u64, u64)> {
    if idx >= count() {
        return None;
    }
    let start = MOD_START[idx].load(Ordering::SeqCst);
    let end = MOD_END[idx].load(Ordering::SeqCst);
    Some((start, end - start))
}

pub fn overlaps(base: u64, size: u64) -> bool {
    let n = count();
    let mut i = 0;
    while i < n {
        let start = MOD_START[i].load(Ordering::SeqCst) & !(PAGE_SIZE - 1);
        let end = (MOD_END[i].load(Ordering::SeqCst) + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        if base < end && start < base + size {
            return true;
        }
        i += 1;
    }
    false
}
