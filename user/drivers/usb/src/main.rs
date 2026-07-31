#![no_std]
#![no_main]

use libsys::{
    OP_BWRITE, OP_PCI_FIND, OP_READ, OP_SYNC, SVC_ATA, SVC_PCI, alloc_dma, map_mmio, mbox_call,
    mbox_connect, print, print_hex, register, serve, vfs_bind,
};

const RING_SIZE: u32 = 256;

const TRB_LINK: u32 = 6;
const TRB_SETUP: u32 = 2;
const TRB_DATA: u32 = 3;
const TRB_STATUS: u32 = 4;
const TRB_NOOP_CMD: u32 = 23;
const TRB_ENABLE_SLOT: u32 = 9;
const TRB_ADDRESS_DEVICE: u32 = 11;
const TRB_CONFIGURE_ENDPOINT: u32 = 12;
const TRB_EVALUATE_CONTEXT: u32 = 13;

const EV_TRANSFER: u32 = 32;
const EV_CMD_COMPLETION: u32 = 33;

fn r32(addr: u64) -> u32 {
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}
fn w32(addr: u64, v: u32) {
    unsafe { core::ptr::write_volatile(addr as *mut u32, v) }
}
fn w64(addr: u64, v: u64) {
    w32(addr, v as u32);
    w32(addr + 4, (v >> 32) as u32);
}
fn rb(addr: u64) -> u8 {
    unsafe { core::ptr::read_volatile(addr as *const u8) }
}
fn wb(addr: u64, v: u8) {
    unsafe { core::ptr::write_volatile(addr as *mut u8, v) }
}

fn dma_zeroed(pages: u64) -> (u64, u64) {
    let (virt, phys) = alloc_dma(pages);
    unsafe {
        core::ptr::write_bytes(virt as *mut u8, 0, (pages * 4096) as usize);
    }
    (virt, phys)
}

fn write_trb(ring: u64, idx: u32, param: u64, status: u32, control: u32) {
    let base = ring + (idx as u64) * 16;
    unsafe {
        core::ptr::write_volatile(base as *mut u64, param);
        core::ptr::write_volatile((base + 8) as *mut u32, status);
        core::ptr::write_volatile((base + 12) as *mut u32, control);
    }
}

struct Ring {
    virt: u64,
    phys: u64,
    enqueue: u32,
    cycle: u32,
}

impl Ring {
    fn alloc() -> Ring {
        let (virt, phys) = dma_zeroed(1);
        Ring {
            virt,
            phys,
            enqueue: 0,
            cycle: 1,
        }
    }

    fn push(&mut self, param: u64, status: u32, control: u32) -> u64 {
        let idx = self.enqueue;
        let phys = self.phys + (idx as u64) * 16;
        write_trb(self.virt, idx, param, status, control | self.cycle);
        self.enqueue += 1;
        if self.enqueue == RING_SIZE - 1 {
            let link = (TRB_LINK << 10) | (1 << 1) | self.cycle;
            write_trb(self.virt, RING_SIZE - 1, self.phys, 0, link);
            self.enqueue = 0;
            self.cycle ^= 1;
        }
        phys
    }
}

struct Xhci {
    op: u64,
    rt: u64,
    db: u64,
    max_slots: u32,
    max_ports: u32,
    csz: u32,
    dcbaa: u64,

    cmd_ring: Ring,

    evt_ring: u64,
    evt_ring_phys: u64,
    evt_dequeue: u32,
    evt_cycle: u32,
    erdp: u64,

    slot: u32,
    ep0_ring: Ring,
    input_ctx: u64,
    input_ctx_phys: u64,
    dev_ctx: u64,

    bulk_in_ring: Ring,
    bulk_out_ring: Ring,
    bulk_in_dci: u32,
    bulk_out_dci: u32,

    scratch: u64,
    scratch_phys: u64,
    data_buf: u64,
    data_phys: u64,
    tag: u32,
}

fn spin_until(addr: u64, mask: u32, want: u32) -> bool {
    let mut budget: u32 = 10_000_000;
    loop {
        if r32(addr) & mask == want {
            return true;
        }
        if budget == 0 {
            return false;
        }
        budget -= 1;
        core::hint::spin_loop();
    }
}

fn legacy_handoff(vbase: u64, hccparams1: u32) {
    let xecp = (hccparams1 >> 16) & 0xFFFF;
    if xecp == 0 {
        return;
    }
    let mut ptr = vbase + (xecp as u64) * 4;
    loop {
        let cap = r32(ptr);
        let id = cap & 0xFF;
        if id == 1 {
            w32(ptr, r32(ptr) | (1 << 24));
            let mut budget: u32 = 10_000_000;
            while r32(ptr) & (1 << 16) != 0 {
                if budget == 0 {
                    print("USB: BIOS handoff timeout\n");
                    break;
                }
                budget -= 1;
                core::hint::spin_loop();
            }
            w32(ptr + 4, 0);
            return;
        }
        let next = (cap >> 8) & 0xFF;
        if next == 0 {
            return;
        }
        ptr += (next as u64) * 4;
    }
}

impl Xhci {
    fn ctx_dw(base: u64, ctx: u32, dw: u32, val: u32, csz: u32) {
        let addr = base + (ctx as u64) * (csz as u64) + (dw as u64) * 4;
        w32(addr, val);
    }

    fn next_event(&mut self, mut budget: u32) -> Option<(u32, u32, u64, u32)> {
        loop {
            let base = self.evt_ring + (self.evt_dequeue as u64) * 16;
            let control = unsafe { core::ptr::read_volatile((base + 12) as *const u32) };
            if control & 1 == self.evt_cycle {
                core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
                let param = unsafe { core::ptr::read_volatile(base as *const u64) };
                let status = unsafe { core::ptr::read_volatile((base + 8) as *const u32) };
                let ttype = (control >> 10) & 0x3F;
                let code = (status >> 24) & 0xFF;
                self.evt_dequeue += 1;
                if self.evt_dequeue == RING_SIZE {
                    self.evt_dequeue = 0;
                    self.evt_cycle ^= 1;
                }
                let deq = self.evt_ring_phys + (self.evt_dequeue as u64) * 16;
                w64(self.erdp, deq | (1 << 3));
                return Some((ttype, code, param, control));
            }
            if budget == 0 {
                return None;
            }
            budget -= 1;
            core::hint::spin_loop();
        }
    }

    fn command(&mut self, param: u64, control: u32) -> Option<(u32, u32)> {
        let trb_phys = self.cmd_ring.push(param, 0, control);
        w32(self.db, 0);
        loop {
            let (ttype, code, evparam, ctrl) = self.next_event(10_000_000)?;
            if ttype == EV_CMD_COMPLETION && evparam == trb_phys {
                return Some((code, ctrl));
            }
        }
    }

    fn reset(&self) -> bool {
        let usbcmd = r32(self.op);
        if usbcmd & 1 != 0 {
            w32(self.op, usbcmd & !1);
            if !spin_until(self.op + 0x04, 1, 1) {
                print("USB: halt timeout\n");
                return false;
            }
        }
        w32(self.op, r32(self.op) | 2);
        let mut budget: u32 = 10_000_000;
        while r32(self.op) & 2 != 0 {
            if budget == 0 {
                print("USB: HCRST timeout\n");
                return false;
            }
            budget -= 1;
            core::hint::spin_loop();
        }
        if !spin_until(self.op + 0x04, 1 << 11, 0) {
            print("USB: CNR timeout\n");
            return false;
        }
        true
    }

    fn noop(&mut self) -> i32 {
        match self.command(0, TRB_NOOP_CMD << 10) {
            Some((code, _)) => code as i32,
            None => -1,
        }
    }

    fn find_port(&self) -> Option<(u32, u32)> {
        let mut best_port = 0u32;
        let mut best_score = 0u32;
        for n in 1..=(self.max_ports) {
            let portsc = self.op + 0x400 + ((n - 1) as u64) * 0x10;
            let val = r32(portsc);
            if val & 1 == 0 {
                continue;
            }
            let speed = (val >> 10) & 0xF;
            let ped = (val >> 1) & 1;
            print("USB: port ");
            print_hex(n);
            print(" portsc=");
            print_hex(val);
            print(" spd=");
            print_hex(speed);
            print(" ped=");
            print_hex(ped);
            print("\n");
            let score = ped * 100 + speed;
            if score > best_score {
                best_score = score;
                best_port = n;
            }
        }
        if best_port == 0 {
            return None;
        }
        let portsc = self.op + 0x400 + ((best_port - 1) as u64) * 0x10;
        let speed = (r32(portsc) >> 10) & 0xF;
        Some((best_port, speed))
    }

    fn reset_port(&self, port: u32) -> bool {
        let portsc = self.op + 0x400 + ((port - 1) as u64) * 0x10;
        if r32(portsc) & (1 << 1) != 0 {
            return true;
        }
        let val = r32(portsc) & !((1 << 1) | (1 << 4) | (1 << 9));
        w32(portsc, val | (1 << 4) | (1 << 9));
        if !spin_until(portsc, 1 << 21, 1 << 21) {
            print("USB: port reset timeout\n");
            return false;
        }
        w32(portsc, r32(portsc) | (1 << 21));
        spin_until(portsc, 1 << 1, 1 << 1)
    }

    fn control(
        &mut self,
        bm: u8,
        brq: u8,
        wval: u16,
        widx: u16,
        wlen: u16,
        data_phys: u64,
        dir_in: bool,
    ) -> Option<u32> {
        let setup = (bm as u64)
            | ((brq as u64) << 8)
            | ((wval as u64) << 16)
            | ((widx as u64) << 32)
            | ((wlen as u64) << 48);
        let trt: u32 = if wlen == 0 {
            0
        } else if dir_in {
            3
        } else {
            2
        };
        self.ep0_ring
            .push(setup, 8, (TRB_SETUP << 10) | (1 << 6) | (trt << 16));
        if wlen > 0 {
            let dir = if dir_in { 1 << 16 } else { 0 };
            self.ep0_ring
                .push(data_phys, wlen as u32, (TRB_DATA << 10) | dir);
        }
        let sdir = if wlen == 0 || !dir_in { 1 << 16 } else { 0 };
        let status_phys = self
            .ep0_ring
            .push(0, 0, (TRB_STATUS << 10) | sdir | (1 << 5));
        w32(self.db + (self.slot as u64) * 4, 1);
        loop {
            match self.next_event(60_000_000) {
                Some((ttype, code, param, _)) => {
                    if ttype == EV_TRANSFER && param == status_phys {
                        return Some(code);
                    }
                }
                None => return None,
            }
        }
    }

    fn get_descriptor(&mut self, dtype: u16, dindex: u16, len: u16, buf_phys: u64) -> Option<u32> {
        self.control(0x80, 6, (dtype << 8) | dindex, 0, len, buf_phys, true)
    }

    fn set_configuration(&mut self, cfg: u16) -> Option<u32> {
        self.control(0x00, 9, cfg, 0, 0, 0, false)
    }

    fn bulk(&mut self, dci: u32, buf_phys: u64, len: u32) -> i32 {
        let trb = if dci == self.bulk_in_dci {
            self.bulk_in_ring.push(buf_phys, len, (1 << 10) | (1 << 5))
        } else {
            self.bulk_out_ring
                .push(buf_phys, len, (1 << 10) | (1 << 5))
        };
        w32(self.db + (self.slot as u64) * 4, dci);
        loop {
            match self.next_event(60_000_000) {
                Some((ttype, code, param, _)) => {
                    if ttype == EV_TRANSFER && param == trb {
                        return code as i32;
                    }
                }
                None => {
                    let ep = self.dev_ctx + (dci as u64) * (self.csz as u64);
                    print("USB: ep wedge dci=");
                    print_hex(dci);
                    print(" state=");
                    print_hex(r32(ep) & 0x7);
                    print(" trdq=");
                    print_hex(r32(ep + 8));
                    print("\n");
                    return -1;
                }
            }
        }
    }

    fn scsi_transfer(&mut self, cb: &[u8], dlen: u32, dir_in: bool, data_phys: u64) -> bool {
        self.tag = self.tag.wrapping_add(1);
        let tag = self.tag;
        let b = self.scratch;
        unsafe {
            core::ptr::write_bytes(b as *mut u8, 0, 31);
        }
        w32(b, 0x43425355);
        w32(b + 4, tag);
        w32(b + 8, dlen);
        wb(b + 12, if dir_in { 0x80 } else { 0 });
        wb(b + 13, 0);
        wb(b + 14, cb.len() as u8);
        for i in 0..cb.len() {
            wb(b + 15 + i as u64, cb[i]);
        }

        let dbg = unsafe {
            SCSI_FAILS += 1;
            SCSI_FAILS <= 3
        };

        let c = self.bulk(self.bulk_out_dci, self.scratch_phys, 31);
        if c < 0 {
            if dbg {
                print("USB: fail@cbw\n");
            }
            return false;
        }
        if dlen > 0 {
            let dci = if dir_in {
                self.bulk_in_dci
            } else {
                self.bulk_out_dci
            };
            let c = self.bulk(dci, data_phys, dlen);
            if c < 0 {
                if dbg {
                    print("USB: fail@data\n");
                }
                return false;
            }
        }

        let csw = self.scratch + 64;
        let csw_phys = self.scratch_phys + 64;
        unsafe {
            core::ptr::write_bytes(csw as *mut u8, 0, 13);
        }
        let c = self.bulk(self.bulk_in_dci, csw_phys, 13);
        if c < 0 {
            if dbg {
                print("USB: fail@csw\n");
            }
            return false;
        }
        let ok = r32(csw) == 0x53425355 && r32(csw + 4) == tag && rb(csw + 12) == 0;
        if !ok && dbg {
            print("USB: csw sig=");
            print_hex(r32(csw));
            print(" tag=");
            print_hex(r32(csw + 4));
            print(" want=");
            print_hex(tag);
            print(" st=");
            print_hex(rb(csw + 12) as u32);
            print("\n");
        }
        ok
    }

    fn unit_ready(&mut self) -> bool {
        let cb = [0u8; 6];
        self.scsi_transfer(&cb, 0, false, 0)
    }

    fn read_capacity(&mut self) -> (u32, u32) {
        let cb = [0x25u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        if !self.scsi_transfer(&cb, 8, true, self.data_phys) {
            return (0, 0);
        }
        let d = self.data_buf;
        let last = ((rb(d) as u32) << 24)
            | ((rb(d + 1) as u32) << 16)
            | ((rb(d + 2) as u32) << 8)
            | (rb(d + 3) as u32);
        let bs = ((rb(d + 4) as u32) << 24)
            | ((rb(d + 5) as u32) << 16)
            | ((rb(d + 6) as u32) << 8)
            | (rb(d + 7) as u32);
        (last, bs)
    }

    fn read_blocks(&mut self, lba: u32, count: u8) -> bool {
        let cb = [
            0x28,
            0,
            (lba >> 24) as u8,
            (lba >> 16) as u8,
            (lba >> 8) as u8,
            lba as u8,
            0,
            0,
            count,
            0,
        ];
        self.scsi_transfer(&cb, count as u32 * 512, true, self.data_phys)
    }

    fn write_blocks(&mut self, lba: u32, count: u8) -> bool {
        let cb = [
            0x2A,
            0,
            (lba >> 24) as u8,
            (lba >> 16) as u8,
            (lba >> 8) as u8,
            lba as u8,
            0,
            0,
            count,
            0,
        ];
        self.scsi_transfer(&cb, count as u32 * 512, false, self.data_phys)
    }

    fn sync_cache(&mut self) -> bool {
        let cb = [0x35u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        self.scsi_transfer(&cb, 0, false, 0)
    }
}

static mut XHCI_PTR: *mut Xhci = core::ptr::null_mut();
static mut SCSI_FAILS: u32 = 0;

fn on_read(req: &[u8], reply: &mut [u8]) -> usize {
    let x = unsafe { &mut *XHCI_PTR };
    let lba = u32::from_le_bytes([req[1], req[2], req[3], req[4]]);
    let mut count = req[5];
    if count > 8 {
        count = 8;
    }
    let n = count as usize * 512;
    if x.read_blocks(lba, count) {
        unsafe {
            core::ptr::copy_nonoverlapping(x.data_buf as *const u8, reply.as_mut_ptr(), n);
        }
    } else {
        print("USB: rd FAIL lba=");
        print_hex(lba);
        print("\n");
    }
    n
}

fn on_bwrite(req: &[u8], reply: &mut [u8]) -> usize {
    let x = unsafe { &mut *XHCI_PTR };
    let lba = u32::from_le_bytes([req[1], req[2], req[3], req[4]]);
    let mut count = req[5];
    if count > 8 {
        count = 8;
    }
    let n = count as usize * 512;
    unsafe {
        core::ptr::copy_nonoverlapping(req.as_ptr().add(6), x.data_buf as *mut u8, n);
    }
    if !x.write_blocks(lba, count) {
        print("USB: wr FAIL lba=");
        print_hex(lba);
        print("\n");
    }
    reply[..4].copy_from_slice(&0i32.to_le_bytes());
    4
}

fn on_sync(_req: &[u8], reply: &mut [u8]) -> usize {
    let x = unsafe { &mut *XHCI_PTR };
    x.sync_cache();
    reply[..4].copy_from_slice(&0i32.to_le_bytes());
    4
}

fn enumerate(x: &mut Xhci) -> bool {
    let (port, speed) = match x.find_port() {
        Some(p) => p,
        None => {
            print("USB: no connected port\n");
            return false;
        }
    };
    print("USB: port=");
    print_hex(port);
    print(" speed=");
    print_hex(speed);
    print("\n");

    if !x.reset_port(port) {
        return false;
    }

    let (code, ctrl) = match x.command(0, TRB_ENABLE_SLOT << 10) {
        Some(v) => v,
        None => {
            print("USB: enable slot no event usbsts=");
            print_hex(r32(x.op + 0x04));
            print(" portsc=");
            print_hex(r32(x.op + 0x400 + ((port - 1) as u64) * 0x10));
            print("\n");
            return false;
        }
    };
    if code != 1 {
        print("USB: enable slot code=");
        print_hex(code);
        print("\n");
        return false;
    }
    x.slot = (ctrl >> 24) & 0xFF;

    let (dev_ctx, dev_ctx_phys) = dma_zeroed(1);
    x.dev_ctx = dev_ctx;
    unsafe {
        core::ptr::write_volatile((x.dcbaa + (x.slot as u64) * 8) as *mut u64, dev_ctx_phys);
    }

    x.ep0_ring = Ring::alloc();

    let mps0: u32 = match speed {
        4 | 5 => 512,
        3 => 64,
        _ => 8,
    };

    let csz = x.csz;
    let ic = x.input_ctx;
    Xhci::ctx_dw(ic, 0, 1, (1 << 0) | (1 << 1), csz);

    Xhci::ctx_dw(ic, 1, 0, (speed << 20) | (1 << 27), csz);
    Xhci::ctx_dw(ic, 1, 1, port << 16, csz);

    Xhci::ctx_dw(ic, 2, 1, (3 << 1) | (4 << 3) | (mps0 << 16), csz);
    let ep0p = x.ep0_ring.phys | 1;
    Xhci::ctx_dw(ic, 2, 2, ep0p as u32, csz);
    Xhci::ctx_dw(ic, 2, 3, (ep0p >> 32) as u32, csz);
    Xhci::ctx_dw(ic, 2, 4, 8, csz);

    let (code, _) = match x.command(x.input_ctx_phys, (TRB_ADDRESS_DEVICE << 10) | (x.slot << 24)) {
        Some(v) => v,
        None => {
            print("USB: address device no event\n");
            return false;
        }
    };
    if code != 1 {
        print("USB: address device code=");
        print_hex(code);
        print("\n");
        return false;
    }

    let (buf, buf_phys) = dma_zeroed(1);

    if x.get_descriptor(1, 0, 8, buf_phys) != Some(1) {
        print("USB: get device desc failed\n");
        return false;
    }
    let real_mps0: u32 = if speed >= 4 {
        1u32 << rb(buf + 7) as u32
    } else {
        rb(buf + 7) as u32
    };
    if real_mps0 != mps0 && real_mps0 != 0 {
        Xhci::ctx_dw(ic, 0, 0, 0, csz);
        Xhci::ctx_dw(ic, 0, 1, 1 << 1, csz);
        Xhci::ctx_dw(ic, 2, 1, (3 << 1) | (4 << 3) | (real_mps0 << 16), csz);
        x.command(x.input_ctx_phys, (TRB_EVALUATE_CONTEXT << 10) | (x.slot << 24));
    }

    if x.get_descriptor(2, 0, 9, buf_phys) != Some(1) {
        print("USB: get config desc header failed\n");
        return false;
    }
    let total = (rb(buf + 2) as u16) | ((rb(buf + 3) as u16) << 8);
    let want = if total as u64 > 4096 { 4096 } else { total as u64 };
    if x.get_descriptor(2, 0, want as u16, buf_phys) != Some(1) {
        print("USB: get config desc failed\n");
        return false;
    }

    let cfg_value = rb(buf + 5) as u16;
    let mut in_addr: u8 = 0;
    let mut out_addr: u8 = 0;
    let mut in_mps: u16 = 0;
    let mut out_mps: u16 = 0;
    let mut off: u64 = 0;
    let mut is_msc = false;
    while off + 2 <= want {
        let blen = rb(buf + off) as u64;
        let btype = rb(buf + off + 1);
        if blen == 0 {
            break;
        }
        if btype == 4 {
            let class = rb(buf + off + 5);
            let sub = rb(buf + off + 6);
            let proto = rb(buf + off + 7);
            is_msc = class == 0x08 && sub == 0x06 && proto == 0x50;
        } else if btype == 5 && is_msc {
            let addr = rb(buf + off + 2);
            let attr = rb(buf + off + 3);
            let mps = (rb(buf + off + 4) as u16) | ((rb(buf + off + 5) as u16) << 8);
            if attr & 0x3 == 2 {
                if addr & 0x80 != 0 {
                    in_addr = addr;
                    in_mps = mps;
                } else {
                    out_addr = addr;
                    out_mps = mps;
                }
            }
        }
        off += blen;
    }

    if in_addr == 0 || out_addr == 0 {
        print("USB: no bulk endpoints found\n");
        return false;
    }

    if x.set_configuration(cfg_value) != Some(1) {
        print("USB: set config failed\n");
        return false;
    }

    x.bulk_in_ring = Ring::alloc();
    x.bulk_out_ring = Ring::alloc();
    x.bulk_in_dci = 2 * (in_addr as u32 & 0x0F) + 1;
    x.bulk_out_dci = 2 * (out_addr as u32 & 0x0F);

    let max_dci = if x.bulk_in_dci > x.bulk_out_dci {
        x.bulk_in_dci
    } else {
        x.bulk_out_dci
    };

    Xhci::ctx_dw(ic, 0, 0, 0, csz);
    Xhci::ctx_dw(
        ic,
        0,
        1,
        (1 << 0) | (1 << x.bulk_in_dci) | (1 << x.bulk_out_dci),
        csz,
    );
    Xhci::ctx_dw(ic, 1, 0, (speed << 20) | (max_dci << 27), csz);
    Xhci::ctx_dw(ic, 1, 1, port << 16, csz);

    let outp = x.bulk_out_ring.phys | 1;
    Xhci::ctx_dw(
        ic,
        1 + x.bulk_out_dci,
        1,
        (3 << 1) | (2 << 3) | ((out_mps as u32) << 16),
        csz,
    );
    Xhci::ctx_dw(ic, 1 + x.bulk_out_dci, 2, outp as u32, csz);
    Xhci::ctx_dw(ic, 1 + x.bulk_out_dci, 3, (outp >> 32) as u32, csz);
    Xhci::ctx_dw(ic, 1 + x.bulk_out_dci, 4, out_mps as u32, csz);

    let inp = x.bulk_in_ring.phys | 1;
    Xhci::ctx_dw(
        ic,
        1 + x.bulk_in_dci,
        1,
        (3 << 1) | (6 << 3) | ((in_mps as u32) << 16),
        csz,
    );
    Xhci::ctx_dw(ic, 1 + x.bulk_in_dci, 2, inp as u32, csz);
    Xhci::ctx_dw(ic, 1 + x.bulk_in_dci, 3, (inp >> 32) as u32, csz);
    Xhci::ctx_dw(ic, 1 + x.bulk_in_dci, 4, in_mps as u32, csz);

    let (code, _) = match x.command(
        x.input_ctx_phys,
        (TRB_CONFIGURE_ENDPOINT << 10) | (x.slot << 24),
    ) {
        Some(v) => v,
        None => {
            print("USB: configure endpoint no event\n");
            return false;
        }
    };
    if code != 1 {
        print("USB: configure endpoint code=");
        print_hex(code);
        print("\n");
        return false;
    }

    print("USB: enumerated slot=");
    print_hex(x.slot);
    print(" in_dci=");
    print_hex(x.bulk_in_dci);
    print(" out_dci=");
    print_hex(x.bulk_out_dci);
    print("\n");
    true
}

fn bringup(vbase: u64) -> Option<Xhci> {
    let cap_len = (r32(vbase) & 0xFF) as u64;
    let hcsparams1 = r32(vbase + 0x04);
    let hcsparams2 = r32(vbase + 0x08);
    let hccparams1 = r32(vbase + 0x10);
    let dboff = r32(vbase + 0x14) & !0x3;
    let rtsoff = r32(vbase + 0x18) & !0x1F;

    let op = vbase + cap_len;
    let rt = vbase + rtsoff as u64;
    let db = vbase + dboff as u64;
    let max_slots = hcsparams1 & 0xFF;
    let max_ports = (hcsparams1 >> 24) & 0xFF;
    let csz = if hccparams1 & (1 << 2) != 0 { 64 } else { 32 };

    let sp_hi = (hcsparams2 >> 21) & 0x1F;
    let sp_lo = (hcsparams2 >> 27) & 0x1F;
    let scratchpad = (sp_hi << 5) | sp_lo;

    print("USB: caplen=");
    print_hex(cap_len as u32);
    print(" max_slots=");
    print_hex(max_slots);
    print(" max_ports=");
    print_hex(max_ports);
    print(" csz=");
    print_hex(csz);
    print(" scratchpad=");
    print_hex(scratchpad);
    print("\n");

    legacy_handoff(vbase, hccparams1);

    let (input_ctx, input_ctx_phys) = dma_zeroed(1);

    let mut xhci = Xhci {
        op,
        rt,
        db,
        max_slots,
        max_ports,
        csz,
        dcbaa: 0,
        cmd_ring: Ring::alloc(),
        evt_ring: 0,
        evt_ring_phys: 0,
        evt_dequeue: 0,
        evt_cycle: 1,
        erdp: rt + 0x20 + 0x18,
        slot: 0,
        ep0_ring: Ring::alloc(),
        input_ctx,
        input_ctx_phys,
        dev_ctx: 0,
        bulk_in_ring: Ring::alloc(),
        bulk_out_ring: Ring::alloc(),
        bulk_in_dci: 0,
        bulk_out_dci: 0,
        scratch: 0,
        scratch_phys: 0,
        data_buf: 0,
        data_phys: 0,
        tag: 0,
    };

    let (scratch, scratch_phys) = dma_zeroed(1);
    let (data_buf, data_phys) = dma_zeroed(1);
    xhci.scratch = scratch;
    xhci.scratch_phys = scratch_phys;
    xhci.data_buf = data_buf;
    xhci.data_phys = data_phys;

    if !xhci.reset() {
        return None;
    }
    print("USB: reset ok usbsts=");
    print_hex(r32(op + 0x04));
    print("\n");

    w32(op + 0x38, max_slots);

    let (dcbaa, dcbaa_phys) = dma_zeroed(1);
    xhci.dcbaa = dcbaa;

    if scratchpad > 0 {
        let (arr, arr_phys) = dma_zeroed(1);
        for i in 0..scratchpad {
            let (_, page_phys) = dma_zeroed(1);
            unsafe {
                core::ptr::write_volatile((arr + (i as u64) * 8) as *mut u64, page_phys);
            }
        }
        unsafe {
            core::ptr::write_volatile(dcbaa as *mut u64, arr_phys);
        }
    }

    w64(op + 0x30, dcbaa_phys);
    w64(op + 0x18, xhci.cmd_ring.phys | 1);

    let (evt_ring, evt_ring_phys) = dma_zeroed(1);
    xhci.evt_ring = evt_ring;
    xhci.evt_ring_phys = evt_ring_phys;

    let (erst, erst_phys) = dma_zeroed(1);
    unsafe {
        core::ptr::write_volatile(erst as *mut u64, evt_ring_phys);
        core::ptr::write_volatile((erst + 8) as *mut u32, RING_SIZE);
        core::ptr::write_volatile((erst + 12) as *mut u32, 0);
    }

    let ir0 = rt + 0x20;
    w32(ir0 + 0x08, 1);
    w64(ir0 + 0x18, evt_ring_phys);
    w64(ir0 + 0x10, erst_phys);

    print("USB: ac64=");
    print_hex(hccparams1 & 1);
    print(" cmdphi=");
    print_hex((xhci.cmd_ring.phys >> 32) as u32);
    print(" evtphi=");
    print_hex((evt_ring_phys >> 32) as u32);
    print(" dcbaphi=");
    print_hex((dcbaa_phys >> 32) as u32);
    print(" sp=");
    print_hex(scratchpad);
    print("\n");

    w32(op, r32(op) | 1);
    if !spin_until(op + 0x04, 1, 0) {
        print("USB: run timeout\n");
        return None;
    }
    print("USB: running usbsts=");
    print_hex(r32(op + 0x04));
    print("\n");

    Some(xhci)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let idx = mbox_connect(SVC_PCI);
    let req = [OP_PCI_FIND, 0x0C, 0x03];
    let mut out = [0u8; 20];
    mbox_call(idx, &req, &mut out);

    if out[0] == 0 {
        print("USB: no xHCI controller found\n");
        loop {}
    }

    let bar0 = u32::from_le_bytes([out[12], out[13], out[14], out[15]]);
    let bar1 = u32::from_le_bytes([out[16], out[17], out[18], out[19]]);
    let phys = ((bar0 as u64) & 0xFFFF_FFF0) | ((bar1 as u64) << 32);

    print("USB: xHCI BAR phys=");
    print_hex((phys >> 32) as u32);
    print_hex(phys as u32);
    print("\n");

    let vbase = map_mmio(phys, 16);

    let mut xhci = match bringup(vbase) {
        Some(x) => x,
        None => {
            print("USB: bringup failed\n");
            loop {}
        }
    };

    let code = xhci.noop();
    print("USB: noop completion code=");
    print_hex(code as u32);
    print("\n");

    if !enumerate(&mut xhci) {
        print("USB: enumerate failed\n");
        loop {}
    }

    unsafe {
        XHCI_PTR = &mut xhci as *mut Xhci;
    }

    let x = unsafe { &mut *XHCI_PTR };
    for _ in 0..5 {
        if x.unit_ready() {
            break;
        }
    }
    let (last, bs) = x.read_capacity();
    print("USB: capacity last_lba=");
    print_hex(last);
    print(" block_size=");
    print_hex(bs);
    print("\n");

    register(OP_READ, on_read);
    register(OP_BWRITE, on_bwrite);
    register(OP_SYNC, on_sync);
    vfs_bind("/dev/ata0".as_bytes(), SVC_ATA);
    print("USB: serving /dev/ata0\n");
    serve(SVC_ATA);

    loop {}
}
