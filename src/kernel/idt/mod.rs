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
    fn isr_33();
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
        IDT[33] = make_entry(isr_33 as *const () as u64);

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
    if vector == 33 {
        crate::io::outb(0x20, 0x20);
        crate::irq::dispatch(1);
        return;
    }
    if vector == 14 {
        let cr2: u64;
        unsafe {
            let rip = *frame.add(17);
            let rsp = *frame.add(20);

            core::arch::asm!("mov {}, cr2", out(reg) cr2);

            crate::serial::write_str("\nPAGE FAULT\ncr2=");
            crate::serial::write_hex(cr2);
            crate::serial::write_str("\n");

            crate::serial::write_str("rip=");
            crate::serial::write_hex(rip);
            crate::serial::write_str("\n");

            crate::serial::write_str("rsp=");
            crate::serial::write_hex(rsp);
            crate::serial::write_str("\n");

            crate::serial::write_str("pid=");
            crate::serial::write_hex((*crate::cpu::get_current_task()).pid as u64);
            crate::serial::write_str("\n");

            let ra = *(rsp as *const u64);
            crate::serial::write_str("ra=");
            crate::serial::write_hex(ra);
            crate::serial::write_str("\n");

            let cr3: u64;
            core::arch::asm!("mov {}, cr3", out(reg) cr3);
            crate::serial::write_str("cr3=");
            crate::serial::write_hex(cr3);
            crate::serial::write_str(" task_cr3=");
            crate::serial::write_hex((*crate::cpu::get_current_task()).cr3);
            crate::serial::write_str(" got380=");
            crate::serial::write_hex(*(0x40f380 as *const u64));
            crate::serial::write_str("\n");

            crate::serial::write_str(" code_ad70=");
            crate::serial::write_hex(*(0x40ad70 as *const u64));
            crate::serial::write_str(" code_0=");
            crate::serial::write_hex(*(0x400000 as *const u64));
            crate::serial::write_str("\n");

            let v = 0x40a000u64;
            let p4 = crate::vmm::phys_to_virt(cr3) as *const u64;
            let e3 = *(crate::vmm::phys_to_virt(*p4.add(((v >> 39) & 0x1ff) as usize) & !0xfff)
                as *const u64)
                .add(((v >> 30) & 0x1ff) as usize);
            let e2 = *(crate::vmm::phys_to_virt(e3 & !0xfff) as *const u64)
                .add(((v >> 21) & 0x1ff) as usize);
            let e1 = *(crate::vmm::phys_to_virt(e2 & !0xfff) as *const u64)
                .add(((v >> 12) & 0x1ff) as usize);
            crate::serial::write_str(" textframe=");
            crate::serial::write_hex(e1 & !0xfff);

            let s = 0x10000000u64;
            let s3 = *(crate::vmm::phys_to_virt(*p4.add(((s >> 39) & 0x1ff) as usize) & !0xfff)
                as *const u64)
                .add(((s >> 30) & 0x1ff) as usize);
            let s2 = *(crate::vmm::phys_to_virt(s3 & !0xfff) as *const u64)
                .add(((s >> 21) & 0x1ff) as usize);
            let s1 = *(crate::vmm::phys_to_virt(s2 & !0xfff) as *const u64)
                .add(((s >> 12) & 0x1ff) as usize);
            crate::serial::write_str(" stackframe=");
            crate::serial::write_hex(s1 & !0xfff);
            crate::serial::write_str(" kstacktop=");
            crate::serial::write_hex((*crate::cpu::get_current_task()).kstack_top);
            crate::serial::write_str("\n");
        }

        loop {}
    }
    crate::serial::write_str("EXCEPTION: ");
    crate::serial::write_hex(vector);
    crate::serial::write_str(" error_code=");
    crate::serial::write_hex(error_code);
    // idk what next just wait ig
    loop {}
}
