use crate::hpet;
use crate::msr::rdmsr;
use crate::vmm::phys_to_virt;

fn base() -> *mut u32 {
    let msr = unsafe { rdmsr(0x1B) };
    phys_to_virt(msr & !0xFFF) as *mut u32
}

fn read(offset: usize) -> u32 {
    unsafe { base().add(offset / 4).read_volatile() }
}

fn write(offset: usize, val: u32) {
    unsafe { base().add(offset / 4).write_volatile(val) }
}

pub fn init() {
    write(0x0F0, read(0x0F0) | 0x1FF);
}
pub fn calibrate(target_hz: u32) -> u32 {
    write(0x3E0, 0x3);
    write(0x380, 0xFFFFFFFF);
    let start = hpet::now_ns();
    while hpet::now_ns() - start < 10_000_000 {}
    let elapsed_ns = hpet::now_ns() - start;
    let count = 0xFFFFFFFFu32 - read(0x390);
    let ticks_per_sec = (count as u64 * 1_000_000_000) / elapsed_ns;
    (ticks_per_sec / target_hz as u64) as u32
}

pub fn init_timer() {
    write(0x3E0, 0x3);
    write(0x320, 0x40 | (1 << 17));
    let count = calibrate(100);
    write(0x380, count);
}

pub fn eoi() {
    write(0x0B0, 0);
}

pub fn send_ipi(apic_id: u8, vector: u8) {
    write(0x310, (apic_id as u32) << 24);
    write(0x300, 0x00004000 | vector as u32);
}

pub fn id() -> u8 {
    (read(0x020) >> 24) as u8
}

pub fn send_init(apic_id: u8) {
    write(0x310, (apic_id as u32) << 24);
    write(0x300, 0x00004500)
}

pub fn send_sipi(apic_id: u8, vector: u8) {
    write(0x310, (apic_id as u32) << 24);
    write(0x300, 0x00004600 | vector as u32);
}
