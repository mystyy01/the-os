#![no_std]
#![no_main]

use libsys::{IPCMessage, inb, syscall};

const SET1: [u8; 0x3A] = [
    0, 0, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 0x08, 0x09, b'q',
    b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', 0, b'a', b's', b'd',
    b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v', b'b',
    b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' ',
];

#[repr(C)]
enum KBButtonState {
    Press,
    Release,
}

#[repr(C)]
struct KBEvent {
    val: u8,
    state: KBButtonState,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() {
    let init_ipcd = unsafe { syscall(13, 1, 0, 0, 0) };
    let irq_ipcd = unsafe { syscall(13, 0, 0, 0, 0) };

    // register for keyboard shit
    unsafe {
        syscall(10, 1, 0, 0, 0);
    }
    let mut msg1 = IPCMessage {
        data: [0; 256],
        len: 0,
    };
    unsafe {
        syscall(7, &mut msg1 as *mut _ as u64, init_ipcd, 0, 0);
    }
    unsafe {
        syscall(12, "im alive!".as_ptr() as u64, 9, init_ipcd, 0);
    }
    loop {
        let mut msg = IPCMessage {
            data: [0; 256],
            len: 0,
        };
        unsafe {
            syscall(7, &mut msg as *mut _ as u64, irq_ipcd, 0, 0);
        }
        unsafe {
            let sc = inb(0x60);
            // actually handle kb presses

            let down = (sc & 0x80) == 0;
            let code = (sc & 0x7F) as usize;
            let val = if code < SET1.len() { SET1[code] } else { 0 };
            if val != 0 {
                let kbev = KBEvent {
                    val: val,
                    state: if down {
                        KBButtonState::Press
                    } else {
                        KBButtonState::Release
                    },
                };
                if down {
                    syscall(8, &val as *const u8 as u64, 1, 0, 0);
                }
            }
        }
    }
}
