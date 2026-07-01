use crate::msr::{rdmsr, wrmsr};
use crate::scheduler::Task;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct CPULocal {
    kernel_stack_top: u64,
    user_rsp: u64,
    kernel_cr3: u64,
    current_task: Option<*mut Task>,
    cpu_id: u32,
}

impl Default for CPULocal {
    fn default() -> Self {
        Self {
            kernel_stack_top: 0,
            user_rsp: 0,
            kernel_cr3: 0,
            current_task: None,
            cpu_id: 0,
        }
    }
}

unsafe impl Sync for CPULocal {}
static mut CPU_LOCALS: [CPULocal; 8] = [CPULocal {
    kernel_stack_top: 0,
    user_rsp: 0,
    kernel_cr3: 0,
    current_task: None,
    cpu_id: 0,
}; 8];
static mut APIC_IDS: [u8; 8] = [0; 8];

pub unsafe fn register_cpu(seq: u32, apic_id: u8) {
    unsafe {
        APIC_IDS[seq as usize] = apic_id;
    }
}

pub unsafe fn apic_id_of(seq: u32) -> u8 {
    unsafe {
        return APIC_IDS[seq as usize];
    }
}

pub unsafe fn index_of(apic_id: u8) -> u32 {
    unsafe {
        for i in 0..8 {
            if APIC_IDS[i] == apic_id {
                return i as u32;
            }
        }
    }
    return 0;
}

pub unsafe fn init(cpu_id: u32, kernel_stack_top: u64) {
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        CPU_LOCALS[cpu_id as usize].kernel_cr3 = cr3;
        CPU_LOCALS[cpu_id as usize].kernel_stack_top = kernel_stack_top;
        CPU_LOCALS[cpu_id as usize].current_task = None;
        CPU_LOCALS[cpu_id as usize].cpu_id = cpu_id;
        wrmsr(0xC0000102, &raw const CPU_LOCALS[cpu_id as usize] as u64);
        wrmsr(0xC0000101, &raw const CPU_LOCALS[cpu_id as usize] as u64);
    }
}

unsafe fn current() -> *mut CPULocal {
    unsafe { rdmsr(0xC0000101) as *mut CPULocal }
}

pub unsafe fn id() -> u32 {
    unsafe {
        return (*current()).cpu_id;
    }
}

pub unsafe fn set_current_task(task: Option<*mut Task>) {
    unsafe {
        (*current()).current_task = task;
    }
}
pub unsafe fn set_stack_top(top: u64) {
    unsafe {
        (*current()).kernel_stack_top = top;
    }
}

pub unsafe fn current_task_opt() -> Option<*mut Task> {
    unsafe {
        return (*current()).current_task;
    }
}

pub unsafe fn get_current_task() -> *mut Task {
    unsafe {
        return (*current()).current_task.unwrap();
    }
}

pub unsafe fn get_kernel_cr3() -> u64 {
    unsafe {
        return (*current()).kernel_cr3;
    }
}
