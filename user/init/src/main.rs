#![no_std]
#![no_main]

use libsys::{SVC_KBD, print, spawn, vfs_bind, vfs_resolve};

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let vfs = include_bytes!("../../dist/vfs.elf");
    spawn(vfs, 1);
    let ata = include_bytes!("../../dist/ata_pio_driver.elf");
    spawn(ata, 0);
    let fs = include_bytes!("../../dist/fs.elf");
    spawn(fs, 0);
    let kbd = include_bytes!("../../dist/kb_driver.elf");
    spawn(kbd, 0);

    vfs_bind(b"/dev/keyboard", SVC_KBD);
    if vfs_resolve(b"/dev/keyboard") == SVC_KBD {
        print("VFS OK\n");
    }

    let shell = include_bytes!("../../dist/shell.elf");
    spawn(shell, 0);
    loop {}
}
