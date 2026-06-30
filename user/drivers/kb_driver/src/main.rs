#![no_std]
#![no_main]

use libsys::{OP_IRQ, OP_READ, SVC_KBD, inb, register, serve, syscall};

const SET1: [u8; 0x3A] = [
    0, 0, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 0x08, 0x09, b'q',
    b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', 0, b'a', b's', b'd',
    b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v', b'b',
    b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' ',
];

const BUF_SIZE: usize = 64;
const EMPTY_EVENT: KBEvent = KBEvent {
    val: 0,
    state: KBButtonState::Release,
};
static mut BUF: [KBEvent; BUF_SIZE] = [EMPTY_EVENT; BUF_SIZE];
static mut HEAD: usize = 0;
static mut TAIL: usize = 0;

#[derive(Copy, Clone)]
#[repr(u8)]
enum KBButtonState {
    Press,
    Release,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct KBEvent {
    val: u8,
    state: KBButtonState,
}
fn buf_push(ev: KBEvent) {
    unsafe {
        let next = (HEAD + 1) % BUF_SIZE;
        if next == TAIL {
            return;
        }
        BUF[HEAD] = ev;
        HEAD = next;
    }
}

fn buf_pop() -> Option<KBEvent> {
    unsafe {
        if HEAD == TAIL {
            return None;
        }
        let b = BUF[TAIL];
        TAIL = (TAIL + 1) % BUF_SIZE;
        Some(b)
    }
}
fn on_read(_req: &[u8], reply: &mut [u8]) -> usize {
    if let Some(ev) = buf_pop() {
        reply[0] = ev.val;
        reply[1] = ev.state as u8;
        2
    } else {
        0
    }
}

fn on_irq(_req: &[u8], _reply: &mut [u8]) -> usize {
    let sc = unsafe { inb(0x60) };
    let down = (sc & 0x80) == 0;
    let code = (sc & 0x7F) as usize;
    let val = if code < SET1.len() { SET1[code] } else { 0 };
    if val != 0 {
        buf_push(KBEvent {
            val,
            state: if down {
                KBButtonState::Press
            } else {
                KBButtonState::Release
            },
        });
    }
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    unsafe {
        // register for keyboard shit
        syscall(10, 1, 0, 0, 0);
    }

    register(OP_IRQ, on_irq);
    register(OP_READ, on_read);

    serve(SVC_KBD);
}
