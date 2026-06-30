#![no_std]
#![no_main]

use libsys::{open, print, read};

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let mut line_buf = [0u8; 256];
    let mut line_len: usize = 0;
    let kb_fd = open("/dev/keyboard".as_bytes());
    loop {
        let mut char_buf = [0u8; 256];
        let n = read(kb_fd, &mut char_buf);
        if n < 2 || char_buf[1] != 0 {
            continue;
        }
        let key = char_buf[0];
        if key == b'\n' {
            let line = &line_buf[..line_len];
            if line == b"hello" {
                print("hi!\n");
            } else if line == b"clear" {
                print("\x1b[2J");
            } else if line.starts_with(b"cat ") {
                let path = &line[4..];
                let fd = open(path);
                if fd < 0 {
                    print("\ncat: not found\n");
                } else {
                    let mut buf = [0u8; 4096];
                    let n = read(fd, &mut buf);
                    print("\n");
                    if n > 0 {
                        print(core::str::from_utf8(&buf[..n as usize]).unwrap_or("?"));
                    }
                }
            } else {
                print("\nunknown command\n");
            }
            line_len = 0;
        } else {
            let one = [key];
            print(core::str::from_utf8(&one).unwrap_or(""));
            if line_len < line_buf.len() {
                line_buf[line_len] = key;
                line_len += 1;
            }
        }
    }
}
