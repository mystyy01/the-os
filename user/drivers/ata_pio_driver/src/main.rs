#![no_std]
#![no_main]

use libsys::{IPCMessage, OP_READ, get_self_pid, inb, inw, outb, print, register, serve, vfs_bind};

fn wait_ready() -> i32 {
    let mut counter: u32 = 100000;
    unsafe {
        loop {
            if counter == 0 {
                return -1;
            }
            let status = inb(0x1F7);
            if status & 0x80 != 0 {
                counter -= 1;
                continue;
            }
            if status & 0x01 != 0 {
                counter -= 1;
                return -1;
            }
            if status & 0x40 != 0 {
                counter -= 1;
                return 0;
            }
            counter -= 1;
        }
    }
}
fn wait_drq() -> i32 {
    let mut c: u32 = 1_000_000;
    unsafe {
        loop {
            if c == 0 {
                return -1;
            }
            let s = inb(0x1F7);
            if s & 0x01 != 0 {
                return -1;
            } // ERR
            if s & 0x80 == 0 && s & 0x08 != 0 {
                return 0;
            } // !BSY && DRQ
            c -= 1;
        }
    }
}
fn delay_400() {
    unsafe {
        inb(0x3F6);
        inb(0x3F6);
        inb(0x3F6);
        inb(0x3F6);
    }
}

fn identify() -> i32 {
    if wait_ready() < 0 {
        return -1;
    }
    unsafe {
        outb(0x1F6, 0xA0);
    }
    delay_400();

    unsafe {
        outb(0x1F2, 0);
        outb(0x1F3, 0);
        outb(0x1F4, 0);
        outb(0x1F5, 0);

        outb(0x1F7, 0xEC);
        let has_drive = inb(0x1F7) != 0;
        if !has_drive {
            return -1;
        }

        if wait_ready() < 0 {
            return -1;
        }
        let mut info: [u16; 256] = [0u16; 256];
        for i in 0..256 {
            info[i] = inw(0x1F0);
        }
    }
    return 0;
}

fn read_sectors(lba: u32, count: u8, buf: &mut [u16]) -> i32 {
    if wait_ready() < 0 {
        return -1;
    }

    unsafe {
        outb(0x1F6, 0xE0 | ((lba >> 24) & 0x0F) as u8);
    }

    delay_400();

    unsafe {
        outb(0x1F2, count);

        outb(0x1F3, (lba & 0xFF) as u8);
        outb(0x1F4, ((lba >> 8) & 0xFF) as u8);
        outb(0x1F5, ((lba >> 16) & 0xFF) as u8);

        outb(0x1F7, 0x20);
    }
    // read the things
    for i in 0..count {
        if wait_drq() < 0 {
            return -1;
        }
        for j in 0..256 {
            unsafe {
                buf[i as usize * 256 + j] = inw(0x1F0);
            }
        }
    }

    return 0;
}

fn on_read(req: &IPCMessage, reply: &mut IPCMessage) {
    let mut buf = [0u16; 2048];
    let buf_bytes =
        unsafe { core::slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 2) };

    let lba = u32::from_le_bytes([req.data[1], req.data[2], req.data[3], req.data[4]]);
    let count = req.data[5];

    read_sectors(lba, count, &mut buf);

    let len = count as usize * 512;
    reply.data[..len].copy_from_slice(&buf_bytes[..len]);
    reply.len = len;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    if identify() < 0 {
        panic!();
    }

    vfs_bind("/dev/ata0".as_bytes(), get_self_pid());

    register(OP_READ, on_read);

    serve();
}
