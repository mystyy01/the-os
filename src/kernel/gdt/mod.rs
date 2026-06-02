unsafe extern "C" {
    static mut tss: u8;
    static tss_end: u8;
    static gdt_start: u8;
    static gdt_end: u8;
}

#[repr(C, packed)]
struct GDTR {
    limit: u16,
    base: u64,
}

pub fn init(kernel_stack_top: u64) {
    let tss_addr = &raw mut tss as u64;
    let tss_size = (&raw const tss_end as u64) - tss_addr;
    unsafe {
        let low = (tss_size & 0xFFFF)
            | ((tss_addr & 0xFFFF) << 16)
            | (((tss_addr >> 16) & 0xFF) << 32)
            | (0x89u64 << 40)
            | (((tss_addr >> 24) & 0xFF) << 56);
        let high = tss_addr >> 32;

        let gdt_base = &raw const gdt_start as *mut u64;
        let tss_slot = gdt_base.add(5);
        *tss_slot = low;
        *tss_slot.add(1) = high;

        let rsp0_ptr = (&raw mut tss as *mut u8).add(4) as *mut u64;
        rsp0_ptr.write_unaligned(kernel_stack_top);

        let iomap_byte = (&raw mut tss as *mut u8).add(104 + 12);
        *iomap_byte = 0xEE;

        let gdtr = GDTR {
            limit: ((&raw const gdt_end as u64) - (&raw const gdt_start as u64) - 1) as u16,
            base: &raw const gdt_start as u64,
        };
        core::arch::asm!("lgdt [{}]", in(reg) &gdtr, options(nostack));
        core::arch::asm!("ltr ax", in("ax") 0x28u16, options(nostack));
    }
}

pub fn set_rsp0(top: u64) {
    unsafe {
        let rsp0_ptr = (&raw mut tss as *mut u8).add(4) as *mut u64;
        rsp0_ptr.write_unaligned(top);
    }
}
