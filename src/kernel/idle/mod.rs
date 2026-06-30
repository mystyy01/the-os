use crate::scheduler::{spawn_idle, spawn_task};

fn idle() {
    loop {
        unsafe {
            core::arch::asm!("sti; hlt");
        }
    }
}

pub fn setup_idle() {
    spawn_idle(idle);
}
