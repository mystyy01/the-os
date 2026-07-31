#![allow(static_mut_refs)]

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const SIZE: usize = 256 * 1024;
const DUMP_TAIL: usize = 48 * 1024;

static mut BUF: [u8; SIZE] = [0u8; SIZE];
static mut CRASH_SNAPSHOT: [u8; SIZE] = [0u8; SIZE];
static mut POS: usize = 0;
static mut WRAPPED: bool = false;
static LOCK: AtomicBool = AtomicBool::new(false);
static DUMPED: AtomicBool = AtomicBool::new(false);
static CRASH_LEN: AtomicUsize = AtomicUsize::new(0);

pub fn append(bytes: &[u8]) {
    while LOCK.swap(true, Ordering::Acquire) {}
    unsafe {
        for &b in bytes {
            BUF[POS] = b;
            POS += 1;
            if POS >= SIZE {
                POS = 0;
                WRAPPED = true;
            }
        }
    }
    LOCK.store(false, Ordering::Release);
}

fn emit_byte(b: u8) {
    crate::serial::write_byte(b);
}

fn emit(s: &str) {
    for b in s.bytes() {
        emit_byte(b);
    }
}

pub fn dump() {
    unsafe {
        let total = if WRAPPED { SIZE } else { POS };
        let take = if total < DUMP_TAIL { total } else { DUMP_TAIL };
        let mut i = (POS + SIZE - take) % SIZE;
        emit("\n===== KLOG TAIL DUMP =====\n");
        let mut n = 0;
        while n < take {
            emit_byte(BUF[i]);
            i += 1;
            if i >= SIZE {
                i = 0;
            }
            n += 1;
        }
        emit("\n===== END KLOG =====\n");
    }
}

pub fn dump_once() {
    if DUMPED.swap(true, Ordering::AcqRel) {
        return;
    }
    dump();
}

fn append_unlocked(bytes: &[u8]) {
    unsafe {
        for &b in bytes {
            BUF[POS] = b;
            POS += 1;
            if POS >= SIZE {
                POS = 0;
                WRAPPED = true;
            }
        }
    }
}

fn append_hex_unlocked(val: u64) {
    append_unlocked(b"0x");
    for i in 0..16 {
        let nibble = ((val >> ((15 - i) * 4)) & 0xf) as u8;
        append_unlocked(&[if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + nibble - 10
        }]);
    }
}

pub fn snapshot_crash(error: u64, cr2: u64, rip: u64, rsp: u64, cpu: u64) {
    let got_lock = LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_ok();
    if got_lock {
        append_unlocked(b"\n===== KERNEL PAGE FAULT =====\nerror=");
        append_hex_unlocked(error);
        append_unlocked(b"\ncr2=");
        append_hex_unlocked(cr2);
        append_unlocked(b"\nrip=");
        append_hex_unlocked(rip);
        append_unlocked(b"\nrsp=");
        append_hex_unlocked(rsp);
        append_unlocked(b"\ncpu=");
        append_hex_unlocked(cpu);
        append_unlocked(b"\n");
    }

    unsafe {
        let total = if WRAPPED { SIZE } else { POS };
        let mut src = if WRAPPED { POS } else { 0 };
        for dst in 0..total {
            CRASH_SNAPSHOT[dst] = BUF[src];
            src += 1;
            if src == SIZE {
                src = 0;
            }
        }
        CRASH_LEN.store(total, Ordering::Release);
    }

    if got_lock {
        LOCK.store(false, Ordering::Release);
    }
}

pub fn read_crash_snapshot(out: *mut u8, max: usize, offset: usize) -> usize {
    let len = CRASH_LEN.load(Ordering::Acquire);
    if out.is_null() || offset >= len {
        return 0;
    }
    let n = core::cmp::min(max, len - offset);
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::addr_of!(CRASH_SNAPSHOT).cast::<u8>().add(offset),
            out,
            n,
        );
    }
    n
}
