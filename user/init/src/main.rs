#![no_std]
#![no_main]

use libsys::{
    OP_BIND, SVC_INIT, SVC_KBD, open, print, read, register, serve, spawn, stop_serving, syscall,
    vfs_bind, vfs_resolve,
};

fn fs_wait(req: &[u8], reply: &mut [u8]) -> usize {
    // just stop waiting now
    stop_serving(SVC_INIT);
    return 0;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let vfs = include_bytes!("../../dist/vfs.elf");
    spawn(vfs, 1);
    let ata = include_bytes!("../../dist/ata_pio_driver.elf");
    spawn(ata, 0);
    let fs = include_bytes!("../../dist/fs.elf");
    spawn(fs, 0);

    // init serves ipc cuz like we had race conditions where fs wasnt mounted when other shit spawned
    register(OP_BIND, fs_wait); // bind is best fit ig
    vfs_bind(b"/run/init", SVC_INIT);
    serve(SVC_INIT);

    const MAX_ELF_SIZE: usize = 256 * 1024;
    let mut scratch = [0u8; MAX_ELF_SIZE];

    let kbd_fd = open(b"/bin/kb_driver");
    if kbd_fd < 0 {
        print("KBD FD FAILED\n");
        loop {}
    }
    let n = read(kbd_fd, &mut scratch);
    spawn(&scratch[..n as usize], 0);

    vfs_bind(b"/dev/keyboard", SVC_KBD);
    if vfs_resolve(b"/dev/keyboard") == SVC_KBD {
        print("VFS OK\n");
    }

    let shell_fd = open(b"/bin/shell");
    if shell_fd < 0 {
        print("SHELL FD FAILED\n");
        loop {}
    }
    let n = read(shell_fd, &mut scratch);
    spawn(&scratch[..n as usize], 0);

    let echo_fd = open(b"/bin/echo");
    if echo_fd < 0 {
        print("ECHO FD FAILED\n");
        loop {}
    }
    let n = read(echo_fd, &mut scratch);
    spawn(&scratch[..n as usize], 2);

    let echo_local_fd = open(b"/bin/echo_local");
    if echo_local_fd < 0 {
        print("ECHO_LOCAL FD FAILED\n");
        loop {}
    }
    let n = read(echo_local_fd, &mut scratch);
    spawn(&scratch[..n as usize], 3);

    let bench_fd = open(b"/bin/bench");
    if bench_fd < 0 {
        print("BENCH FD FAILED\n");
        loop {}
    }
    let n = read(bench_fd, &mut scratch);
    spawn(&scratch[..n as usize], 3);

    loop {
        unsafe {
            syscall(9, 0, 0, 0, 0);
        }
    }
    loop {}
}
