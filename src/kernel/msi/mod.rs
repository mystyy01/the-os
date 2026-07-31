use core::sync::atomic::{AtomicU64, Ordering};

static COUNT: AtomicU64 = AtomicU64::new(0);
static WAKER: AtomicU64 = AtomicU64::new(0);

pub fn set_waker(ident: u64) {
    WAKER.store(ident, Ordering::Release);
}

pub fn take() -> u64 {
    COUNT.swap(0, Ordering::AcqRel)
}

pub fn interrupt() {
    COUNT.fetch_add(1, Ordering::Release);
    let ident = WAKER.load(Ordering::Acquire);
    if ident != 0 {
        crate::sleepq::wakeup(ident);
    }
}
