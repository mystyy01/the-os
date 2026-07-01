#![no_std]
#![no_main]

use libsys::{
    OP_ECHO, OP_ECHO_TS, PP_BASE, PP_ITERS, PP_WARMUP, SVC_ECHO, mbox_call, mbox_call_prof,
    mbox_call_spin, mbox_connect, print, rdtsc,
};

const WARMUP: u64 = 256;
const ITERS: u64 = 10_000;

static mut REQ: [u8; 4096] = [0u8; 4096];
static mut OUT: [u8; 4096] = [0u8; 4096];

type Call = fn(usize, &[u8], &mut [u8]) -> usize;

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

fn kv(k: &str, v: u64) {
    print(k);
    print_u64(v);
}

fn measure(idx: usize, call: Call, len: usize) -> (u64, u64, u64) {
    let req = unsafe { &mut *core::ptr::addr_of_mut!(REQ) };
    let out = unsafe { &mut *core::ptr::addr_of_mut!(OUT) };
    req[0] = OP_ECHO;

    for _ in 0..WARMUP {
        call(idx, &req[..len], out);
    }

    let mut min = u64::MAX;
    let mut max = 0u64;
    let mut sum = 0u64;
    for _ in 0..ITERS {
        let t0 = rdtsc();
        call(idx, &req[..len], out);
        let t1 = rdtsc();
        let d = t1 - t0;
        if d < min {
            min = d;
        }
        if d > max {
            max = d;
        }
        sum += d;
    }
    (min, sum / ITERS, max)
}

fn row(label: &str, idx: usize, call: Call, len: usize) {
    let (mn, av, mx) = measure(idx, call, len);
    print(label);
    kv("  min=", mn);
    kv(" avg=", av);
    kv(" max=", mx);
    print("\n");
}

fn floor() {
    let word = PP_BASE as *mut u32;
    let srv = (PP_BASE + 8) as *mut u64;

    let mut rt_min = u64::MAX;
    let mut rt_sum = 0u64;
    let mut out_sum = 0u64;
    let mut in_sum = 0u64;

    let mut v = 0u32;
    for i in 0..PP_WARMUP + PP_ITERS {
        v += 1;
        let req = v * 2 - 1;
        let t0 = rdtsc();
        unsafe { core::ptr::write_volatile(word, req) };
        while unsafe { core::ptr::read_volatile(word) } != req + 1 {
            core::hint::spin_loop();
        }
        let t1 = rdtsc();
        let st = unsafe { core::ptr::read_volatile(srv) };

        if i >= PP_WARMUP {
            let rt = t1 - t0;
            if rt < rt_min {
                rt_min = rt;
            }
            rt_sum += rt;
            out_sum += st.wrapping_sub(t0);
            in_sum += t1.wrapping_sub(st);
        }
    }

    print("--- floor: bare cache-line ping-pong ---\n");
    kv("  rt   min=", rt_min);
    kv(" avg=", rt_sum / PP_ITERS);
    print("\n");
    kv("  outbound avg=", out_sum / PP_ITERS);
    kv("   inbound avg=", in_sum / PP_ITERS);
    print("\n");
}

fn staged(idx: usize) {
    let req = unsafe { &mut *core::ptr::addr_of_mut!(REQ) };
    let out = unsafe { &mut *core::ptr::addr_of_mut!(OUT) };
    req[0] = OP_ECHO_TS;

    for _ in 0..WARMUP {
        mbox_call_prof(idx, &req[..16], out);
    }

    let mut outb = 0u64;
    let mut srvw = 0u64;
    let mut inb = 0u64;
    let mut rt = 0u64;
    for _ in 0..ITERS {
        let (t1, t4) = mbox_call_prof(idx, &req[..16], out);
        let t2 = u64::from_le_bytes([out[0], out[1], out[2], out[3], out[4], out[5], out[6], out[7]]);
        let t3 = u64::from_le_bytes([
            out[8], out[9], out[10], out[11], out[12], out[13], out[14], out[15],
        ]);
        outb = outb.wrapping_add(t2.wrapping_sub(t1));
        srvw = srvw.wrapping_add(t3.wrapping_sub(t2));
        inb = inb.wrapping_add(t4.wrapping_sub(t3));
        rt += t4 - t1;
    }

    print("--- staged: real echo path (avg cycles) ---\n");
    kv("  outbound(req->srv saw)=", outb / ITERS);
    print("\n");
    kv("  server work=", srvw / ITERS);
    kv("   inbound(srv->cli saw)=", inb / ITERS);
    print("\n");
    kv("  total(t4-t1)=", rt / ITERS);
    print("\n");
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    print("bench: up\n");
    floor();

    let idx = mbox_connect(SVC_ECHO);
    staged(idx);

    print("=== IPC round-trip (TSC cycles, cross-core) ===\n");
    row("8B    busy ", idx, mbox_call_spin, 8);
    row("8B    yield", idx, mbox_call, 8);
    row("512B  busy ", idx, mbox_call_spin, 512);
    row("512B  yield", idx, mbox_call, 512);
    row("4096B busy ", idx, mbox_call_spin, 4096);
    row("4096B yield", idx, mbox_call, 4096);
    print("=== bench done ===\n");

    loop {}
}
