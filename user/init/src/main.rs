#![no_std]
#![no_main]

use libsys::{IPCMessage, print, spawn, syscall, vfs_bind, vfs_resolve};

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    unsafe {
        // start vfs here (HAS TO BE == VFS_PID in the libsys file i think)
        let vfs = include_bytes!("../../dist/vfs.elf");
        let vfs_pid = spawn(vfs);

        let kbd = include_bytes!("../../dist/kb_driver.elf");
        let kbd_pid = spawn(kbd);
        let kbd_ipcd = syscall(13, kbd_pid as u64, 0, 0, 0);
        let mut msg: IPCMessage = IPCMessage {
            data: [0; 256],
            len: 0,
        };

        let r = vfs_bind(b"/dev/keyboard", kbd_pid);
        let got = vfs_resolve(b"/dev/keyboard");

        if got == kbd_pid {
            print("VFS OK")
        }

        let shell = include_bytes!("../../dist/shell.elf");
        let shell_pid = spawn(shell);

        loop {}
    }
}
