use crate::cpu::get_current_task;
use crate::scheduler::{self, Task};
use core::sync::atomic::{AtomicBool, Ordering};

const MAX_SLEEPERS: usize = 64;

#[derive(Copy, Clone)]
struct SleepEntry {
    ident: u64,
    task: *mut Task,
    deadline_ns: u64,
    outcome: u64,
    active: bool,
}

static mut TABLE: [SleepEntry; MAX_SLEEPERS] = [SleepEntry {
    ident: 0,
    task: core::ptr::null_mut(),
    deadline_ns: 0,
    outcome: 0,
    active: false,
}; MAX_SLEEPERS];

static LOCK: AtomicBool = AtomicBool::new(false);

fn lock() {
    while LOCK.swap(true, Ordering::Acquire) {}
}

fn unlock() {
    LOCK.store(false, Ordering::Release);
}

pub const WOKEN: u64 = 0;
pub const TIMED_OUT: u64 = 1;

pub fn sleep(ident: u64, timeout_ns: u64) -> u64 {
    unsafe {
        let cur = get_current_task();

        lock();
        let mut idx: usize = usize::MAX;
        for i in 0..MAX_SLEEPERS {
            if !TABLE[i].active {
                idx = i;
                break;
            }
        }
        if idx == usize::MAX {
            unlock();
            return TIMED_OUT;
        }
        let deadline = if timeout_ns > 0 {
            crate::hpet::now_ns() + timeout_ns
        } else {
            0
        };
        TABLE[idx] = SleepEntry {
            ident,
            task: cur,
            deadline_ns: deadline,
            outcome: WOKEN,
            active: true,
        };
        unlock();

        scheduler::block_current();

        lock();
        let outcome = TABLE[idx].outcome;
        TABLE[idx].active = false;
        unlock();
        outcome
    }
}

pub fn wakeup(ident: u64) {
    unsafe {
        lock();
        for i in 0..MAX_SLEEPERS {
            if TABLE[i].active && TABLE[i].ident == ident {
                TABLE[i].outcome = WOKEN;
                scheduler::wake(Some(TABLE[i].task));
            }
        }
        unlock();
    }
}

pub fn tick() {
    unsafe {
        let now = crate::hpet::now_ns();
        lock();
        for i in 0..MAX_SLEEPERS {
            if TABLE[i].active && TABLE[i].deadline_ns != 0 && now >= TABLE[i].deadline_ns {
                TABLE[i].outcome = TIMED_OUT;
                scheduler::wake(Some(TABLE[i].task));
                TABLE[i].deadline_ns = 0;
            }
        }
        unlock();
    }
}
