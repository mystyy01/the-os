#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use libsys::{SLEEP_TIMED_OUT, SLEEP_WOKEN, print, spawn_thread, sys_sleep, sys_wakeup};

const WORKERS: usize = 4;
const STACK_ORDER: u64 = 3;
const PRIORITY: u64 = 1;
const SHORT_NS: u64 = 100_000;
const LONG_NS: u64 = 50_000_000;
const WAIT_LIMIT: u64 = 200_000_000;
const SOAK_ROUNDS: u32 = 4096;

static COMMAND_GEN: AtomicU32 = AtomicU32::new(0);
static COMMAND_MASK: AtomicU32 = AtomicU32::new(0);
static STARTED: AtomicU32 = AtomicU32::new(0);
static ARMED: AtomicU32 = AtomicU32::new(0);
static DONE: AtomicU32 = AtomicU32::new(0);
static IDENT: [AtomicU64; WORKERS] = [const { AtomicU64::new(0) }; WORKERS];
static TIMEOUT: [AtomicU64; WORKERS] = [const { AtomicU64::new(0) }; WORKERS];
static RESULT: [AtomicU64; WORKERS] = [const { AtomicU64::new(u64::MAX) }; WORKERS];
static RETURNS: [AtomicU32; WORKERS] = [const { AtomicU32::new(0) }; WORKERS];
const BURN_MODE: u64 = u64::MAX;
static BURN: AtomicU32 = AtomicU32::new(0);

fn number(mut n: u64) {
    let mut buf = [0u8; 20];
    let mut p = buf.len();
    if n == 0 {
        print("0");
        return;
    }
    while n != 0 {
        p -= 1;
        buf[p] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    print(core::str::from_utf8(&buf[p..]).unwrap_or("?"));
}

fn line(test: &str, pass: bool, detail: &str, value: u64) {
    print("PSTRESS test=");
    print(test);
    print(if pass {
        " status=PASS "
    } else {
        " status=FAIL "
    });
    print(detail);
    print("=");
    number(value);
    print("\n");
}

fn heartbeat(phase: &str, round: u32) {
    print("PSTRESS heartbeat phase=");
    print(phase);
    print(" round=");
    number(round as u64);
    print("\n");
}

fn wait_bits(word: &AtomicU32, mask: u32) -> bool {
    let mut n = 0;
    while word.load(Ordering::Acquire) & mask != mask {
        core::hint::spin_loop();
        n += 1;
        if n == WAIT_LIMIT {
            return false;
        }
    }
    true
}

fn dispatch(mask: u32, ids: [u64; WORKERS], timeouts: [u64; WORKERS]) -> u32 {
    ARMED.store(0, Ordering::Release);
    DONE.store(0, Ordering::Release);
    for i in 0..WORKERS {
        IDENT[i].store(ids[i], Ordering::Relaxed);
        TIMEOUT[i].store(timeouts[i], Ordering::Relaxed);
        RESULT[i].store(u64::MAX, Ordering::Relaxed);
        RETURNS[i].store(0, Ordering::Relaxed);
    }
    COMMAND_MASK.store(mask, Ordering::Relaxed);
    COMMAND_GEN.fetch_add(1, Ordering::Release) + 1
}

fn worker(slot: usize) -> ! {
    STARTED.fetch_or(1 << slot, Ordering::Release);
    let mut seen = 0;
    loop {
        let generation = COMMAND_GEN.load(Ordering::Acquire);
        if generation == seen {
            core::hint::spin_loop();
            continue;
        }
        seen = generation;
        let bit = 1 << slot;
        if COMMAND_MASK.load(Ordering::Relaxed) & bit == 0 {
            continue;
        }
        let id = IDENT[slot].load(Ordering::Relaxed);
        let timeout = TIMEOUT[slot].load(Ordering::Relaxed);
        ARMED.fetch_or(bit, Ordering::Release);
        if timeout == BURN_MODE {
            while BURN.load(Ordering::Acquire) != 0 {
                core::hint::spin_loop();
            }
            RESULT[slot].store(SLEEP_TIMED_OUT, Ordering::Relaxed);
            RETURNS[slot].fetch_add(1, Ordering::Relaxed);
            DONE.fetch_or(bit, Ordering::Release);
            continue;
        }
        let result = sys_sleep(id, timeout);
        RESULT[slot].store(result, Ordering::Relaxed);
        RETURNS[slot].fetch_add(1, Ordering::Relaxed);
        DONE.fetch_or(bit, Ordering::Release);
    }
}

extern "C" fn worker0() -> ! {
    worker(0)
}
extern "C" fn worker1() -> ! {
    worker(1)
}
extern "C" fn worker2() -> ! {
    worker(2)
}
extern "C" fn worker3() -> ! {
    worker(3)
}

fn valid(slot: usize) -> bool {
    let r = RESULT[slot].load(Ordering::Acquire);
    (r == SLEEP_WOKEN || r == SLEEP_TIMED_OUT) && RETURNS[slot].load(Ordering::Acquire) == 1
}

fn wake_until(id: u64, mask: u32) -> bool {
    let mut n = 0;
    while DONE.load(Ordering::Acquire) & mask != mask {
        sys_wakeup(id);
        n += 1;
        if n == WAIT_LIMIT {
            return false;
        }
    }
    true
}

libsys::entry!(main);

unsafe extern "C" fn main() -> ! {
    let entries = [worker0 as extern "C" fn() -> !, worker1, worker2, worker3];
    let mut spawned = true;
    for entry in entries {
        spawned &= spawn_thread(entry, STACK_ORDER, PRIORITY) >= 0;
    }
    let startup = spawned && wait_bits(&STARTED, 0xf);
    line(
        "thread_startup_liveness",
        startup,
        "started_mask",
        STARTED.load(Ordering::Acquire) as u64,
    );
    let mut all = startup;

    let id_a = 0x5053_1001;
    let id_b = 0x5053_1002;
    dispatch(0b0011, [id_a, id_b, 0, 0], [LONG_NS, SHORT_NS, 0, 0]);
    let armed = wait_bits(&ARMED, 0b0011);
    let woke = wake_until(id_a, 0b0001);
    let completed = wait_bits(&DONE, 0b0011);
    let isolated = armed
        && woke
        && completed
        && RESULT[0].load(Ordering::Acquire) == SLEEP_WOKEN
        && RESULT[1].load(Ordering::Acquire) == SLEEP_TIMED_OUT
        && valid(0)
        && valid(1);
    line(
        "unique_id_targeted_wake_isolation",
        isolated,
        "returns",
        (RETURNS[0].load(Ordering::Acquire) + RETURNS[1].load(Ordering::Acquire)) as u64,
    );
    all &= isolated;

    let broadcast_id = 0x5053_2001;
    dispatch(
        0b0111,
        [broadcast_id, broadcast_id, broadcast_id, 0],
        [LONG_NS, LONG_NS, LONG_NS, 0],
    );
    let broadcast = wait_bits(&ARMED, 0b0111)
        && wake_until(broadcast_id, 0b0111)
        && valid(0)
        && valid(1)
        && valid(2)
        && RESULT[0].load(Ordering::Acquire) == SLEEP_WOKEN
        && RESULT[1].load(Ordering::Acquire) == SLEEP_WOKEN
        && RESULT[2].load(Ordering::Acquire) == SLEEP_WOKEN;
    line(
        "same_id_broadcast_wake",
        broadcast,
        "woken",
        DONE.load(Ordering::Acquire).count_ones() as u64,
    );
    all &= broadcast;

    let nonsticky_id = 0x5053_3001;
    sys_wakeup(nonsticky_id);
    dispatch(1, [nonsticky_id, 0, 0, 0], [SHORT_NS, 0, 0, 0]);
    let nonsticky =
        wait_bits(&DONE, 1) && valid(0) && RESULT[0].load(Ordering::Acquire) == SLEEP_TIMED_OUT;
    line(
        "wake_before_sleep_non_stickiness",
        nonsticky,
        "result",
        RESULT[0].load(Ordering::Acquire),
    );
    all &= nonsticky;

    let mut timeout_ok = true;
    for round in 0..256u32 {
        dispatch(1, [0x5053_4001, 0, 0, 0], [SHORT_NS, 0, 0, 0]);
        timeout_ok &=
            wait_bits(&DONE, 1) && valid(0) && RESULT[0].load(Ordering::Acquire) == SLEEP_TIMED_OUT;
        if round & 63 == 63 {
            heartbeat("short_timeout", round + 1);
        }
    }
    line("repeated_short_timeout_progress", timeout_ok, "rounds", 256);
    all &= timeout_ok;

    let mut race_ok = true;
    for phase in 0..64u32 {
        let id = 0x5053_5000 + phase as u64;
        dispatch(1, [id, 0, 0, 0], [SHORT_NS, 0, 0, 0]);
        race_ok &= wait_bits(&ARMED, 1);
        for _ in 0..(phase * 32) {
            core::hint::spin_loop();
        }
        sys_wakeup(id);
        race_ok &= wait_bits(&DONE, 1) && valid(0);
        if phase & 15 == 15 {
            heartbeat("wake_timeout_sweep", phase + 1);
        }
    }
    line(
        "wake_vs_timeout_deterministic_phase_sweep",
        race_ok,
        "phases",
        64,
    );
    all &= race_ok;

    let mut soak_ok = true;
    for round in 0..SOAK_ROUNDS {
        let wake_id = 0x5053_6000 + (round & 7) as u64;
        let timeout_id = 0x5053_7000 + (round & 7) as u64;
        dispatch(
            0b0011,
            [wake_id, timeout_id, 0, 0],
            [LONG_NS, SHORT_NS, 0, 0],
        );
        soak_ok &=
            wait_bits(&ARMED, 0b0011) && wake_until(wake_id, 0b0001) && wait_bits(&DONE, 0b0011);
        soak_ok &= valid(0)
            && valid(1)
            && RESULT[0].load(Ordering::Acquire) == SLEEP_WOKEN
            && RESULT[1].load(Ordering::Acquire) == SLEEP_TIMED_OUT;
        if round & 511 == 511 {
            heartbeat("mixed_soak", round + 1);
        }
    }
    line(
        "mixed_sustained_soak",
        soak_ok,
        "rounds",
        SOAK_ROUNDS as u64,
    );
    all &= soak_ok;

    let mut starvation_ok = true;
    for round in 0..128u32 {
        BURN.store(1, Ordering::Release);
        dispatch(
            0b1110,
            [0, 0xdead, 0xbeef, 0xf00d],
            [0, BURN_MODE, BURN_MODE, BURN_MODE],
        );
        wait_bits(&ARMED, 0b1110);
        let mut n = 0u64;
        loop {
            let r = sys_sleep(0x5053_8000 + round as u64, SHORT_NS);
            let ok = r == SLEEP_TIMED_OUT;
            starvation_ok &= ok;
            n += 1;
            if n >= 1 || !ok {
                break;
            }
        }
        BURN.store(0, Ordering::Release);
        wait_bits(&DONE, 0b1110);
        if round & 15 == 15 {
            heartbeat("timeout_under_runnable_load", round + 1);
        }
    }
    line(
        "timed_sleep_progress_under_runnable_load",
        starvation_ok,
        "rounds",
        128,
    );
    all &= starvation_ok;

    print(if all {
        "PSTRESS FINAL status=PASS\n"
    } else {
        "PSTRESS FINAL status=FAIL\n"
    });
    loop {
        core::hint::spin_loop();
    }
}
