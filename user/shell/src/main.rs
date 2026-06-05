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
            match &line_buf[..line_len] {
                b"hello" => print("hi!\n"),
                b"clear" => print("\x1b[2J"),
                _ => print("\nunknown command\n"),
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
