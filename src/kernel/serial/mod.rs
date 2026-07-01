use crate::io::inb;
use crate::io::outb;
use core::sync::atomic::{AtomicBool, Ordering};

static SERIAL_LOCK: AtomicBool = AtomicBool::new(false);

pub fn lock() {
    while SERIAL_LOCK.swap(true, Ordering::Acquire) {}
}

pub fn unlock() {
    SERIAL_LOCK.store(false, Ordering::Release);
}

pub fn init() {
    outb(0x3F8 + 1, 0x00); // this is interrups disable
    outb(0x3F8 + 3, 0x80); // sets up baud rate (what is baud rate)
    outb(0x3F8 + 0, 0x01); // baud
    outb(0x3F8 + 1, 0x00); // more baud 
    outb(0x3F8 + 3, 0x03); // baud is a weird word
    outb(0x3F8 + 2, 0xC7); // fifo
    outb(0x3F8 + 4, 0x0B); // irq
}

pub fn write_byte(byte: u8) {
    while inb(0x3F8 + 5) & 0x20 == 0 {}
    outb(0x3F8, byte);
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

pub fn write_hex(val: u64) {
    write_str("0x");
    for i in 0..16 {
        let nibble = (val >> ((15 - i) * 4)) & 0x0F;
        let byte = if nibble < 10 {
            b'0' + nibble as u8
        } else {
            b'a' + (nibble - 10) as u8
        };
        write_byte(byte);
    }
}
