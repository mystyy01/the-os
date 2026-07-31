use crate::io;
use crate::pmm;
use crate::serial::{write_hex, write_str};
use crate::vmm::phys_to_virt;
use core::sync::atomic::{Ordering, fence};

const DCID: u64 = 0x00;
const DCDB: u64 = 0x04;
const DCERSTSZ: u64 = 0x08;
const DCERSTBA: u64 = 0x10;
const DCERDP: u64 = 0x18;
const DCCTRL: u64 = 0x20;
const DCST: u64 = 0x24;
const DCPORTSC: u64 = 0x28;
const DCCP: u64 = 0x30;
const DCDDI1: u64 = 0x38;
const DCDDI2: u64 = 0x3C;

const CTRL_DCE: u32 = 1 << 31;
const CTRL_LSE: u32 = 1 << 1;
const CTRL_DCR: u32 = 1 << 0;

const TRB_LINK: u32 = 6 << 10;
const TRB_CYCLE: u32 = 1 << 0;
const TRB_TOGGLE: u32 = 1 << 1;

const RING_TRBS: u64 = 256;

static mut DBC: u64 = 0;
static mut MEM_PHYS: u64 = 0;

const OFF_CTX: u64 = 0x0000;
const OFF_ERST: u64 = 0x0400;
const OFF_EVT: u64 = 0x1000;
const OFF_OUT: u64 = 0x2000;
const OFF_IN: u64 = 0x3000;
const OFF_STR: u64 = 0x4000;
const OFF_TX: u64 = 0x5000;

fn pci_cfg_read32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let addr: u32 = 0x8000_0000
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((off as u32) & 0xFC);
    io::outl(0xCF8, addr);
    io::inl(0xCFC)
}

fn find_xhci() -> Option<u64> {
    for bus in 0u8..=255 {
        for dev in 0u8..32 {
            for func in 0u8..8 {
                if pci_cfg_read32(bus, dev, func, 0x00) == 0xFFFF_FFFF {
                    continue;
                }
                let class = pci_cfg_read32(bus, dev, func, 0x08);
                if (class >> 24) & 0xFF == 0x0C && (class >> 16) & 0xFF == 0x03 {
                    let bar0 = pci_cfg_read32(bus, dev, func, 0x10);
                    let bar1 = pci_cfg_read32(bus, dev, func, 0x14);
                    return Some(((bar0 as u64) & 0xFFFF_FFF0) | ((bar1 as u64) << 32));
                }
            }
        }
    }
    None
}

fn rd(off: u64) -> u32 {
    unsafe { core::ptr::read_volatile((DBC + off) as *const u32) }
}

fn wr(off: u64, val: u32) {
    unsafe { core::ptr::write_volatile((DBC + off) as *mut u32, val) }
}

fn wr64(off: u64, val: u64) {
    wr(off, val as u32);
    wr(off + 4, (val >> 32) as u32);
}

fn mem_virt(off: u64) -> u64 {
    unsafe { phys_to_virt(MEM_PHYS + off) }
}

fn mem_phys(off: u64) -> u64 {
    unsafe { MEM_PHYS + off }
}

fn wdw(off: u64, val: u32) {
    unsafe { core::ptr::write_volatile(mem_virt(off) as *mut u32, val) }
}

fn build_string(off: u64, s: &str) -> u8 {
    let base = mem_virt(off) as *mut u8;
    let len = 2 + s.len() * 2;
    unsafe {
        *base = len as u8;
        *base.add(1) = 0x03;
        let mut i = 2;
        for c in s.bytes() {
            *base.add(i) = c;
            *base.add(i + 1) = 0;
            i += 2;
        }
    }
    len as u8
}

fn build_strings() -> (u8, u8, u8, u8) {
    let base = mem_virt(OFF_STR) as *mut u8;
    unsafe {
        *base = 4;
        *base.add(1) = 0x03;
        *base.add(2) = 0x09;
        *base.add(3) = 0x04;
    }
    let l0 = 4u8;
    let lm = build_string(OFF_STR + 0x10, "theos");
    let lp = build_string(OFF_STR + 0x30, "theos dbg");
    let ls = build_string(OFF_STR + 0x60, "0001");
    (l0, lm, lp, ls)
}

fn init_ring(off: u64) {
    for i in 0..RING_TRBS {
        wdw(off + i * 16, 0);
        wdw(off + i * 16 + 4, 0);
        wdw(off + i * 16 + 8, 0);
        wdw(off + i * 16 + 12, 0);
    }
    let last = (RING_TRBS - 1) * 16;
    let ring_phys = mem_phys(off);
    wdw(off + last, ring_phys as u32);
    wdw(off + last + 4, (ring_phys >> 32) as u32);
    wdw(off + last + 8, 0);
    wdw(off + last + 12, TRB_LINK | TRB_TOGGLE | TRB_CYCLE);
}

fn build_ep(ctx_off: u64, ep_type: u32, ring_off: u64) {
    let ring_phys = mem_phys(ring_off);
    wdw(ctx_off, 0);
    wdw(ctx_off + 4, (3 << 1) | (ep_type << 3) | (1024 << 16));
    wdw(ctx_off + 8, (ring_phys as u32) | 1);
    wdw(ctx_off + 12, (ring_phys >> 32) as u32);
    wdw(ctx_off + 16, 1024);
    wdw(ctx_off + 20, 0);
    wdw(ctx_off + 24, 0);
    wdw(ctx_off + 28, 0);
}

pub fn init() {
    let phys = match find_xhci() {
        Some(x) => x,
        None => {
            write_str("xdbc: no xHCI found\n");
            return;
        }
    };

    let mmio = phys_to_virt(phys);
    let hccparams1 = unsafe { core::ptr::read_volatile((mmio + 0x10) as *const u32) };
    let xecp = ((hccparams1 >> 16) & 0xFFFF) as u64;
    if xecp == 0 {
        write_str("xdbc: no extended caps\n");
        return;
    }

    let mut off = xecp * 4;
    let mut dbc_off: u64 = 0;
    for _ in 0..64 {
        let cap = unsafe { core::ptr::read_volatile((mmio + off) as *const u32) };
        if cap & 0xFF == 0x0A {
            dbc_off = off;
            break;
        }
        let next = (cap >> 8) & 0xFF;
        if next == 0 {
            break;
        }
        off += (next as u64) * 4;
    }
    if dbc_off == 0 {
        write_str("xdbc: DbC not found\n");
        return;
    }

    let mem = pmm::alloc_pages(3) as u64;
    unsafe {
        DBC = mmio + dbc_off;
        MEM_PHYS = mem;
    }
    for i in 0..(0x8000 / 4) {
        wdw(i * 4, 0);
    }

    let (l0, lm, lp, ls) = build_strings();

    let s0 = mem_phys(OFF_STR);
    let sm = mem_phys(OFF_STR + 0x10);
    let sp = mem_phys(OFF_STR + 0x30);
    let ss = mem_phys(OFF_STR + 0x60);
    wdw(OFF_CTX + 0x00, s0 as u32);
    wdw(OFF_CTX + 0x04, (s0 >> 32) as u32);
    wdw(OFF_CTX + 0x08, sm as u32);
    wdw(OFF_CTX + 0x0C, (sm >> 32) as u32);
    wdw(OFF_CTX + 0x10, sp as u32);
    wdw(OFF_CTX + 0x14, (sp >> 32) as u32);
    wdw(OFF_CTX + 0x18, ss as u32);
    wdw(OFF_CTX + 0x1C, (ss >> 32) as u32);
    wdw(
        OFF_CTX + 0x20,
        (l0 as u32) | ((lm as u32) << 8) | ((lp as u32) << 16) | ((ls as u32) << 24),
    );

    init_ring(OFF_OUT);
    init_ring(OFF_IN);

    for i in 0..RING_TRBS {
        wdw(OFF_EVT + i * 16, 0);
        wdw(OFF_EVT + i * 16 + 4, 0);
        wdw(OFF_EVT + i * 16 + 8, 0);
        wdw(OFF_EVT + i * 16 + 12, 0);
    }
    let evt_phys = mem_phys(OFF_EVT);
    wdw(OFF_ERST + 0, evt_phys as u32);
    wdw(OFF_ERST + 4, (evt_phys >> 32) as u32);
    wdw(OFF_ERST + 8, RING_TRBS as u32);
    wdw(OFF_ERST + 12, 0);

    build_ep(OFF_CTX + 0x40, 2, OFF_OUT);
    build_ep(OFF_CTX + 0x80, 6, OFF_IN);

    fence(Ordering::SeqCst);

    wr(DCERSTSZ, 1);
    wr64(DCERSTBA, mem_phys(OFF_ERST));
    wr64(DCERDP, mem_phys(OFF_EVT));
    wr64(DCCP, mem_phys(OFF_CTX));
    wr(DCDDI1, (0x1d6b << 16) | 0x0000);
    wr(DCDDI2, (0x0010 << 16) | 0x0010);

    fence(Ordering::SeqCst);

    wr(DCCTRL, CTRL_DCE | CTRL_LSE);

    fence(Ordering::SeqCst);

    write_str("xdbc: enabled, DCID=");
    write_hex(rd(DCID) as u64);
    write_str("\n");

    let mut spins: u64 = 0;
    while rd(DCCTRL) & CTRL_DCR == 0 {
        spins += 1;
        if spins > 100_000_000 {
            break;
        }
    }

    write_str("xdbc: DCCTRL=");
    write_hex(rd(DCCTRL) as u64);
    write_str(" DCST=");
    write_hex(rd(DCST) as u64);
    write_str(" DCPORTSC=");
    write_hex(rd(DCPORTSC) as u64);
    write_str("\n");

    let mut last_port = rd(DCPORTSC);
    write_str("xdbc: waiting for host (plug cable)...\n");
    loop {
        let p = rd(DCPORTSC);
        if p != last_port {
            last_port = p;
            write_str("xdbc: DCPORTSC=");
            write_hex(p as u64);
            write_str(" DCCTRL=");
            write_hex(rd(DCCTRL) as u64);
            write_str(" DCST=");
            write_hex(rd(DCST) as u64);
            write_str("\n");
        }
    }
}
