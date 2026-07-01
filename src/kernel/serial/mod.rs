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

const VGA_W: usize = 80;
const VGA_H: usize = 25;
static mut VGA_POS: usize = 0;

fn vga_putc(byte: u8) {
    unsafe {
        let vga = crate::vmm::phys_to_virt(0xb8000) as *mut u16;
        if byte == b'\n' {
            VGA_POS = (VGA_POS / VGA_W + 1) * VGA_W;
        } else if byte == b'\r' {
            VGA_POS = (VGA_POS / VGA_W) * VGA_W;
        } else {
            *vga.add(VGA_POS) = 0x0f00 | byte as u16;
            VGA_POS += 1;
        }
        if VGA_POS >= VGA_W * VGA_H {
            for i in 0..VGA_W * (VGA_H - 1) {
                *vga.add(i) = *vga.add(i + VGA_W);
            }
            for i in VGA_W * (VGA_H - 1)..VGA_W * VGA_H {
                *vga.add(i) = 0x0f00 | b' ' as u16;
            }
            VGA_POS = VGA_W * (VGA_H - 1);
        }
    }
}

pub fn write_byte(byte: u8) {
    while inb(0x3F8 + 5) & 0x20 == 0 {}
    outb(0x3F8, byte);
    if crate::fb::present() {
        crate::fb::con_putc(byte);
    } else {
        vga_putc(byte);
    }
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
