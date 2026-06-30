use core::sync::atomic::{AtomicBool, Ordering};

static PMM_LOCK: AtomicBool = AtomicBool::new(false);

pub fn lock() {
    while PMM_LOCK.swap(true, Ordering::Acquire) {}
    return;
}

pub fn unlock() {
    PMM_LOCK.store(false, Ordering::Release);
}
