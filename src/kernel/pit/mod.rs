use crate::{io::outb, scheduler, serial::write_str};

pub fn init(hz: u32) {
    let divisor: u32 = 1193182 / hz;
    outb(0x43, 0x36);
    let div_low = (divisor & 0xFF) as u8;
    let div_high = ((divisor >> 8) & 0xFF) as u8;

    outb(0x40, div_low);
    outb(0x40, div_high);
}

pub fn irq0_handler(frame: *mut u64) {
    outb(0x20, 0x20);
    unsafe {
        // scheduler::schedule(frame);
    }
}
