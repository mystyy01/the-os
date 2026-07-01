use crate::lapic;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct Tss {
    reserved0: u32,
    rsp: [u64; 3],
    reserved1: u64,
    ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    iomap_base: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CpuGdt {
    entries: [u64; 7],
    tss: Tss,
}

#[repr(C, packed)]
struct Gdtr {
    limit: u16,
    base: u64,
}

const TSS_SIZE: u16 = core::mem::size_of::<Tss>() as u16;
const MAX_CPUS: usize = 8;

static mut CPU_GDTS: [CpuGdt; MAX_CPUS] = [CpuGdt {
    entries: [
        0x0000000000000000,
        0x00AF9A000000FFFF,
        0x00AF92000000FFFF,
        0x00AFF2000000FFFF,
        0x00AFFA000000FFFF,
        0,
        0,
    ],
    tss: Tss {
        reserved0: 0,
        rsp: [0; 3],
        reserved1: 0,
        ist: [0; 7],
        reserved2: 0,
        reserved3: 0,
        iomap_base: TSS_SIZE,
    },
}; MAX_CPUS];

pub fn init(cpu_id: u32, kernel_stack_top: u64) {
    unsafe {
        let gdt = &mut CPU_GDTS[cpu_id as usize];
        let tss_addr = core::ptr::addr_of!(gdt.tss) as u64;
        let tss_limit = (TSS_SIZE - 1) as u64;

        gdt.entries[5] = tss_limit & 0xFFFF
            | ((tss_addr & 0xFFFF) << 16)
            | (((tss_addr >> 16) & 0xFF) << 32)
            | (0x89u64 << 40)
            | (((tss_addr >> 24) & 0xFF) << 56);
        gdt.entries[6] = tss_addr >> 32;

        let rsp0 = (core::ptr::addr_of_mut!(gdt.tss) as *mut u8).add(4) as *mut u64;
        core::ptr::write_unaligned(rsp0, kernel_stack_top);

        let gdtr = Gdtr {
            limit: (core::mem::size_of::<[u64; 7]>() - 1) as u16,
            base: core::ptr::addr_of!(gdt.entries) as u64,
        };
        core::arch::asm!("lgdt [{}]", in(reg) &gdtr, options(nostack));
        core::arch::asm!("ltr ax", in("ax") 0x28u16, options(nostack));
    }
}

pub fn set_rsp0(top: u64) {
    unsafe {
        let tss = core::ptr::addr_of_mut!(CPU_GDTS[crate::cpu::id() as usize].tss);
        let rsp0 = (tss as *mut u8).add(4) as *mut u64;
        core::ptr::write_unaligned(rsp0, top);
    }
}
