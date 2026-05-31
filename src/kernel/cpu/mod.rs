use crate::msr::{rdmsr, wrmsr};
use crate::scheduler::Task;

#[repr(C, packed)]
pub struct CPULocal {
    kernel_stack_top: u64,
    user_rsp: u64,
    current_task: Option<*mut Task>,
    cpu_id: u32,
}
unsafe impl Sync for CPULocal {}
static mut CPU_LOCAL: CPULocal = CPULocal {
    kernel_stack_top: 0,
    user_rsp: 0,
    current_task: None,
    cpu_id: 0,
};

pub unsafe fn init(cpu_id: u32, kernel_stack_top: u64) {
    unsafe {
        CPU_LOCAL.kernel_stack_top = kernel_stack_top;
        CPU_LOCAL.current_task = None;
        CPU_LOCAL.cpu_id = cpu_id;
        wrmsr(0xC0000102, &raw const CPU_LOCAL as u64)
    }
}
