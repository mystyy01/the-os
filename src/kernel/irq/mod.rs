use core::sync::atomic::{Ordering, fence};

use crate::{
    ipc::{ARENA_PHYS, BUFPOOL_OFF, MBOX_OFF, MBOX_REQ, Mailbox, OP_IRQ},
    vmm,
};

static mut IRQ_HANDLERS: [i32; 16] = [-1; 16];

pub fn register(irq: usize, service_id: u32) {
    if irq < 16 {
        unsafe {
            IRQ_HANDLERS[irq] = service_id as i32;
        }
        crate::ipc::inbox_add(service_id, (48 + irq) as u32);
    }
}

pub fn dispatch(irq: usize) {
    unsafe {
        if irq >= 16 {
            return;
        }
        let service_id = IRQ_HANDLERS[irq];
        if service_id < 0 {
            return;
        }

        if irq == 1 {
            let sc = crate::io::inb(0x60);
            crate::ipc::irq_ring_push(irq, sc);
        }

        let idx = 48 + irq;
        let mbox_phys = ARENA_PHYS + MBOX_OFF as u64 + (idx * 64) as u64;
        let buf_phys = ARENA_PHYS + BUFPOOL_OFF as u64 + (idx * 4096) as u64;
        let mbox = vmm::phys_to_virt(mbox_phys) as *mut Mailbox;
        let buf = vmm::phys_to_virt(buf_phys) as *mut u8;

        *buf = OP_IRQ;
        (*mbox).client_id = 0;
        (*mbox).service_id = service_id as u32;
        (*mbox).msg_offset = (BUFPOOL_OFF + idx * 4096) as u32;
        (*mbox).len = 1;
        fence(Ordering::Release);
        (*mbox).status = MBOX_REQ;

        crate::ipc::wake_server(service_id as u32);
    }
}
