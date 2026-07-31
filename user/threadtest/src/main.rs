#![no_std]
#![no_main]

use libsys::{SLEEP_TIMED_OUT, SLEEP_WOKEN, print, spawn_thread, sys_sleep, sys_wakeup};

const WAKE_IDENT: u64 = 0xC0FFEE;
const TIMEOUT_IDENT: u64 = 0xDEAD10CC;
const TWO_SECONDS_NS: u64 = 2_000_000_000;

fn print_u64(mut n: u64) {
    if n == 0 {
        print("0");
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 20;
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    print(core::str::from_utf8(&buf[i..]).unwrap_or("?"));
}

fn spin_delay() {
    let mut i: u64 = 0;
    while i < 30_000_000 {
        core::hint::spin_loop();
        i += 1;
    }
}

extern "C" fn thread_entry() -> ! {
    print("  [thread] alive, ticking 3 times before waking main\n");
    let mut n: u64 = 0;
    while n < 3 {
        print("  [thread] tick ");
        print_u64(n);
        print(" (main should be SILENT right now)\n");
        n += 1;
        spin_delay();
    }
    print("  [thread] calling sys_wakeup(WAKE_IDENT) now\n");
    sys_wakeup(WAKE_IDENT);

    loop {
        print("  [thread] idle tick\n");
        spin_delay();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    print("[main] alive, spawning thread\n");
    let tid = spawn_thread(thread_entry, 2, 1);
    print("[main] spawn_thread returned tid=");
    print_u64(tid as u64);
    print("\n");

    print("[main] sleeping (no timeout) on WAKE_IDENT - should print NOTHING until thread wakes me\n");
    let result = sys_sleep(WAKE_IDENT, 0);
    print("[main] woke up! result=");
    print_u64(result);
    print(if result == SLEEP_WOKEN {
        " (WOKEN, correct)\n"
    } else {
        " (WRONG - expected WOKEN)\n"
    });

    print("[main] now testing real timeout: sleeping 2s on an ident nobody will ever wake\n");
    let result = sys_sleep(TIMEOUT_IDENT, TWO_SECONDS_NS);
    print("[main] timeout sleep returned result=");
    print_u64(result);
    print(if result == SLEEP_TIMED_OUT {
        " (TIMED_OUT, correct)\n"
    } else {
        " (WRONG - expected TIMED_OUT)\n"
    });

    print("[main] test complete, idling\n");
    let mut n: u64 = 0;
    loop {
        print("[main] idle tick ");
        print_u64(n);
        print("\n");
        n += 1;
        spin_delay();
    }
}
