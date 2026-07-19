#![no_std]
#![no_main]

use libsys::{
    OP_BIND, OP_PCI_FIND, SVC_INIT, SVC_KBD, SVC_PCI, mbox_call, mbox_connect, open, print,
    print_hex, read, register, serve, spawn, stop_serving, syscall, vfs_bind, vfs_resolve,
};

fn fs_wait(req: &[u8], reply: &mut [u8]) -> usize {
    // just stop waiting now
    stop_serving(SVC_INIT);
    return 0;
}

fn pci_probe_debug() {
    let idx = mbox_connect(SVC_PCI);
    let req = [OP_PCI_FIND, 0x03, 0xFF];
    let mut out = [0u8; 20];
    mbox_call(idx, &req, &mut out);
    if out[0] == 0 {
        print("PCI: no class 0x03 device found\n");
        return;
    }
    let vendor = u16::from_le_bytes([out[4], out[5]]);
    let dev_id = u16::from_le_bytes([out[6], out[7]]);
    let bar0 = u32::from_le_bytes([out[12], out[13], out[14], out[15]]);
    let bar1 = u32::from_le_bytes([out[16], out[17], out[18], out[19]]);
    print("PCI: display dev bus=0 device=");
    print_hex(out[2] as u32);
    print(" vendor=");
    print_hex(vendor as u32);
    print(" device=");
    print_hex(dev_id as u32);
    print(" bar0=");
    print_hex(bar0);
    print(" bar1=");
    print_hex(bar1);
    print("\n");
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let vfs = include_bytes!("../../dist/vfs.elf");
    spawn(vfs, 1);
    let ata = include_bytes!("../../dist/ata.elf");
    spawn(ata, 0);
    let fs = include_bytes!("../../dist/fs.elf");
    spawn(fs, 0);
    let pci = include_bytes!("../../dist/pci.elf");
    spawn(pci, 0);
    let usb = include_bytes!("../../dist/usb.elf");
    spawn(usb, 0);

    pci_probe_debug();

    // init serves ipc cuz like we had race conditions where fs wasnt mounted when other shit spawned
    register(OP_BIND, fs_wait); // bind is best fit ig
    vfs_bind(b"/run/init", SVC_INIT);
    serve(SVC_INIT);

    const MAX_ELF_SIZE: usize = 256 * 1024;
    let mut scratch = [0u8; MAX_ELF_SIZE];

    let kbd_fd = open(b"/bin/kbd");
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
