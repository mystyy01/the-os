use crate::pit;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IDTEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    attr: u8,
    offset_mid: u16,
    offset_high: u32,
    _reserved: u32,
}

static mut IDT: [IDTEntry; 256] = [IDTEntry {
    offset_low: 0,
    selector: 0,
    ist: 0,
    attr: 0,
    offset_mid: 0,
    offset_high: 0,
    _reserved: 0,
}; 256];

fn make_entry(handler: u64) -> IDTEntry {
    let entry = IDTEntry {
        offset_low: handler as u16,
        selector: 0x08,
        ist: 0,
        attr: 0x8E,
        offset_mid: (handler >> 16) as u16,
        offset_high: (handler >> 32) as u32,
        _reserved: 0,
    };
    return entry;
}

unsafe extern "C" {
    fn isr_0();
    fn isr_6();
    fn isr_8();
    fn isr_13();
    fn isr_14();
    fn isr_32();
}
#[repr(C, packed)]
struct IDTR {
    limit: u16,
    base: u64,
}
pub fn init() {
    unsafe {
        IDT[0] = make_entry(isr_0 as *const () as u64);
        IDT[6] = make_entry(isr_6 as *const () as u64);
        IDT[8] = make_entry(isr_8 as *const () as u64);
        IDT[13] = make_entry(isr_13 as *const () as u64);
        IDT[14] = make_entry(isr_14 as *const () as u64);
        IDT[32] = make_entry(isr_32 as *const () as u64);

        let idtr: IDTR = IDTR {
            limit: (256 * 16 - 1) as u16,
            base: core::ptr::addr_of!(IDT) as u64,
        };

        core::arch::asm!("lidt [{}]", in(reg) &idtr, options(nostack));
    }
}

#[unsafe(no_mangle)]
extern "C" fn exception_handler(vector: u64, error_code: u64, frame: *mut u64) {
    if vector == 32 {
        pit::irq0_handler(frame);
        return;
    }
    crate::serial::write_str("EXCEPTION: ");
    crate::serial::write_hex(vector);
    crate::serial::write_str(" error_code=");
    crate::serial::write_hex(error_code);
    // idk what next just wait ig
    loop {}
}
