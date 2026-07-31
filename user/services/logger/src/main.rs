#![no_std]
#![no_main]

use libsys::{close, create, create_trunc, sys_sleep, ulog_drain, write};

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let mut buf = [0u8; 4096];
    let mut first = true;
    loop {
        let n = ulog_drain(&mut buf);
        if n > 0 {
            let fd = if first {
                create_trunc(b"/userspace.log")
            } else {
                create(b"/userspace.log")
            };
            if fd >= 0 {
                first = false;
                write(fd, &buf[..n]);
                loop {
                    let m = ulog_drain(&mut buf);
                    if m == 0 {
                        break;
                    }
                    write(fd, &buf[..m]);
                }
                close(fd);
            }
        }
        sys_sleep(0x106601, 20_000_000);
    }
}
