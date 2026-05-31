use crate::msr::{rdmsr, wrmsr};
use crate::scheduler::Task;

#[repr(C, packed)]
pub struct CPULocal {
    kernel_stack_top: u64,
    user_rsp: u64,
    kernel_cr3: u64,
    current_task: Option<*mut Task>,
    cpu_id: u32,
}
unsafe impl Sync for CPULocal {}
static mut CPU_LOCAL: CPULocal = CPULocal {
    kernel_stack_top: 0,
    user_rsp: 0,
    kernel_cr3: 0,
    current_task: None,
    cpu_id: 0,
};

pub unsafe fn init(cpu_id: u32, kernel_stack_top: u64) {
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        CPU_LOCAL.kernel_cr3 = cr3;
        CPU_LOCAL.kernel_stack_top = kernel_stack_top;
        CPU_LOCAL.current_task = None;
        CPU_LOCAL.cpu_id = cpu_id;
        wrmsr(0xC0000102, &raw const CPU_LOCAL as u64)
    }
}

pub unsafe fn set_current_task(task: Option<*mut Task>) {
    unsafe {
        CPU_LOCAL.current_task = task;
    }
}

pub unsafe fn get_current_task() -> *mut Task {
    unsafe {
        return CPU_LOCAL.current_task.unwrap();
    }
}

pub unsafe fn get_kernel_cr3() -> u64 {
    unsafe {
        return CPU_LOCAL.kernel_cr3;
    }
}
