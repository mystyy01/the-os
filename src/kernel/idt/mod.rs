use crate::cpu;
use crate::lapic;
use crate::pit;
use crate::pmm;
use crate::scheduler;
use crate::scheduler::cleanup_and_exit_task;
use crate::vmm;

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
    fn isr_64();
    fn isr_65();
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
        IDT[64] = make_entry(isr_64 as *const () as u64);
        IDT[65] = make_entry(isr_65 as *const () as u64);

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
    if vector == 64 {
        lapic::eoi();
        crate::sleepq::tick();
        unsafe {
            if cpu::current_task_opt().is_none() {
                return;
            }
        }
        if scheduler::try_direct_wake() {
            return;
        }
        scheduler::yield_now();
        return;
    }
    if vector == 65 {
        lapic::eoi();
        crate::msi::interrupt();
        return;
    }
    unsafe {
        if *frame.add(18) & 3 == 3 {
            let curr_task = cpu::get_current_task();
            let rip = *frame.add(17);
            let rsp = *frame.add(20);

            crate::klog::dump_once();
            crate::serial::write_str("\nRING3 CRASH\npid=");
            crate::serial::write_hex((*curr_task).pid as u64);
            crate::serial::write_str(" vector=");
            crate::serial::write_hex(vector);
            crate::serial::write_str(" error_code=");
            crate::serial::write_hex(error_code);
            crate::serial::write_str("\n");

            crate::serial::write_str("rip=");
            crate::serial::write_hex(rip);
            crate::serial::write_str(" rsp=");
            crate::serial::write_hex(rsp);
            crate::serial::write_str("\n");

            let ra = *(rsp as *const u64);
            crate::serial::write_str("ra=");
            crate::serial::write_hex(ra);
            crate::serial::write_str("\n");

            crate::serial::write_str("stack code candidates:");
            for i in 0..32 {
                let candidate = *((rsp as *const u64).add(i));
                if (0x400000..0x700000).contains(&candidate) {
                    crate::serial::write_str(" ");
                    crate::serial::write_hex(candidate);
                }
            }
            crate::serial::write_str("\n");

            if vector == 14 {
                let cr2: u64;
                core::arch::asm!("mov {}, cr2", out(reg) cr2);
                crate::serial::write_str("cr2=");
                crate::serial::write_hex(cr2);
                crate::serial::write_str("\n");

                crate::klog::snapshot_crash(
                    error_code,
                    cr2,
                    rip,
                    rsp,
                    crate::lapic::id() as u64,
                );
                let _ = crate::ipc::notify_kernel_crash(9);
            }

            let cr3: u64;
            core::arch::asm!("mov {}, cr3", out(reg) cr3);
            crate::serial::write_str("cr3=");
            crate::serial::write_hex(cr3);
            crate::serial::write_str(" task_cr3=");
            crate::serial::write_hex((*curr_task).cr3);
            crate::serial::write_str("\n");

            cleanup_and_exit_task(curr_task);
            return;
        }
    }
    if vector == 14 {
        let cr2: u64;
        unsafe {
            let rip = *frame.add(17);
            let rsp = *frame.add(20);

            core::arch::asm!("mov {}, cr2", out(reg) cr2);

            let first = crate::serial::begin_crash_output();
            if !first {
                crate::serial::write_str_raw("\nNESTED PAGE FAULT\n");
            } else {
                crate::serial::write_str_raw("KERNEL PAGE FAULT\n");
            }
            crate::serial::write_str_raw("error=");
            crate::serial::write_hex_raw(error_code);
            crate::serial::write_str_raw("\ncr2=");
            crate::serial::write_hex_raw(cr2);
            crate::serial::write_str_raw("\nrip=");
            crate::serial::write_hex_raw(rip);
            crate::serial::write_str_raw("\nrsp=");
            crate::serial::write_hex_raw(rsp);
            crate::serial::write_str_raw("\ncpu=");
            let cpu_id = crate::lapic::id() as u64;
            crate::serial::write_hex_raw(cpu_id);
            crate::serial::write_str_raw("\n");

            crate::klog::snapshot_crash(error_code, cr2, rip, rsp, cpu_id);
            if crate::ipc::notify_kernel_crash(9) {
                crate::scheduler::kill_current_task();
            }
        }

        loop {}
    }
    crate::serial::begin_crash_output();
    crate::serial::write_str("EXCEPTION: ");
    crate::serial::write_hex(vector);
    crate::serial::write_str(" error_code=");
    crate::serial::write_hex(error_code);
    unsafe {
        let rip = *frame.add(17);
        crate::serial::write_str(" rip=");
        crate::serial::write_hex(rip);
        crate::serial::write_str(" cpu=");
        crate::serial::write_hex(crate::lapic::id() as u64);
        crate::serial::write_str("\n");
    }
    loop {}
}
