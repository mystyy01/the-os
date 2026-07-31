#![no_std]
#![no_main]

use libsys::{
    OP_KERNEL_CRASH, SVC_CRASHLOG, close, create_trunc, kernel_crashlog_read, register, serve,
    write,
};

fn on_kernel_crash(_req: &[u8], _reply: &mut [u8]) -> usize {
    let fd = create_trunc(b"/kernel.log");
    if fd < 0 {
        return 0;
    }

    let mut buf = [0u8; 4096];
    let mut offset = 0usize;
    loop {
        let n = kernel_crashlog_read(&mut buf, offset);
        if n == 0 {
            break;
        }
        if write(fd, &buf[..n]) < 0 {
            break;
        }
        offset += n;
    }
    close(fd);
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    register(OP_KERNEL_CRASH, on_kernel_crash);
    serve(SVC_CRASHLOG);
    loop {}
}
