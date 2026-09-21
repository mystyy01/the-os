use crate::cpu::get_current_task;
use crate::scheduler::{self, MAX_CPUS, Task};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const MAX_SLEEPERS: usize = 64;

#[derive(Copy, Clone)]
struct SleepEntry {
    ident: u64,
    task: *mut Task,
    deadline_ns: u64,
    outcome: u64,
    active: bool,
}

const EMPTY_ENTRY: SleepEntry = SleepEntry {
    ident: 0,
    task: core::ptr::null_mut(),
    deadline_ns: 0,
    outcome: 0,
    active: false,
};

static mut TABLES: [[SleepEntry; MAX_SLEEPERS]; MAX_CPUS] =
    [[EMPTY_ENTRY; MAX_SLEEPERS]; MAX_CPUS];

#[allow(clippy::declare_interior_mutable_const)]
const LOCK_INIT: AtomicBool = AtomicBool::new(false);
static LOCKS: [AtomicBool; MAX_CPUS] = [LOCK_INIT; MAX_CPUS];

#[allow(clippy::declare_interior_mutable_const)]
const COUNT_INIT: AtomicU32 = AtomicU32::new(0);
static ACTIVE: [AtomicU32; MAX_CPUS] = [COUNT_INIT; MAX_CPUS];

fn lock(q: usize) -> u64 {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            "cli",
            out(reg) flags,
            options(nomem)
        );
    }
    while LOCKS[q].swap(true, Ordering::Acquire) {}
    flags
}

fn unlock(q: usize, flags: u64) {
    LOCKS[q].store(false, Ordering::Release);
    if flags & (1 << 9) != 0 {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }
}

fn table(q: usize) -> &'static mut [SleepEntry; MAX_SLEEPERS] {
    unsafe { &mut *(&raw mut TABLES[q]) }
}

fn this_cpu() -> usize {
    let id = unsafe { crate::cpu::id() } as usize;
    debug_assert!(id < MAX_CPUS);
    id % MAX_CPUS
}

pub const WOKEN: u64 = 0;
pub const TIMED_OUT: u64 = 1;

pub fn sleep(ident: u64, timeout_ns: u64) -> u64 {
    let h = prepare(ident, timeout_ns);
    commit(h)
}

pub fn prepare(ident: u64, timeout_ns: u64) -> u64 {
    unsafe {
        let cur = get_current_task();
        let q = this_cpu();

        let flags = lock(q);
        let t = table(q);
        let mut idx: usize = usize::MAX;
        for i in 0..MAX_SLEEPERS {
            if !t[i].active {
                idx = i;
                break;
            }
        }
        if idx == usize::MAX {
            unlock(q, flags);
            return u64::MAX;
        }
        let deadline = if timeout_ns > 0 {
            crate::hpet::now_ns() + timeout_ns
        } else {
            0
        };
        t[idx] = SleepEntry {
            ident,
            task: cur,
            deadline_ns: deadline,
            outcome: WOKEN,
            active: true,
        };
        ACTIVE[q].fetch_add(1, Ordering::Release);
        unlock(q, flags);
        ((q as u64) << 32) | idx as u64
    }
}

pub fn commit(handle: u64) -> u64 {
    if handle == u64::MAX {
        return TIMED_OUT;
    }
    unsafe {
        let q = (handle >> 32) as usize;
        let idx = (handle & 0xFFFF_FFFF) as usize;
        if q >= MAX_CPUS || idx >= MAX_SLEEPERS {
            return TIMED_OUT;
        }

        scheduler::block_current();

        let flags = lock(q);
        let t = table(q);
        let outcome = t[idx].outcome;
        if t[idx].active {
            t[idx].active = false;
            ACTIVE[q].fetch_sub(1, Ordering::Release);
        }
        unlock(q, flags);
        outcome
    }
}

pub fn wakeup(ident: u64) {
    for q in 0..MAX_CPUS {
        if ACTIVE[q].load(Ordering::Acquire) == 0 {
            continue;
        }
        let flags = lock(q);
        let t = table(q);
        for i in 0..MAX_SLEEPERS {
            if t[i].active && t[i].ident == ident {
                t[i].outcome = WOKEN;
                scheduler::wake(Some(t[i].task));
            }
        }
        unlock(q, flags);
    }
}

pub fn tick() {
    let q = this_cpu();
    if ACTIVE[q].load(Ordering::Acquire) == 0 {
        return;
    }
    let now = crate::hpet::now_ns();
    let flags = lock(q);
    let t = table(q);
    for i in 0..MAX_SLEEPERS {
        if t[i].active && t[i].deadline_ns != 0 && now >= t[i].deadline_ns {
            t[i].outcome = TIMED_OUT;
            scheduler::wake(Some(t[i].task));
            t[i].deadline_ns = 0;
        }
    }
    unlock(q, flags);
}
