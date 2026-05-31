use crate::scheduler::spawn_task;

fn idle() {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn setup_idle() {
    spawn_task(idle, 0);
}
