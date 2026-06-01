use crate::serial::{write_byte, write_hex, write_str};

pub struct FileOps {
    pub write: fn(*mut OpenFile, *const u8, usize) -> i64,
    pub read: fn(*mut OpenFile, *mut u8, usize) -> i64,
}

unsafe impl Sync for OpenFile {}

pub struct OpenFile {
    pub ops: *const FileOps,
    offset: u64,
    private: u64,
}

fn serial_write(_: *mut OpenFile, buf: *const u8, len: usize) -> i64 {
    unsafe {
        for i in 0..len {
            write_byte(*buf.add(i));
        }
        return 0;
    };
}

fn serial_read(_: *mut OpenFile, _: *mut u8, _: usize) -> i64 {
    return -1;
}

static SERIAL_OPS: FileOps = FileOps {
    write: serial_write,
    read: serial_read,
};

pub static mut CONSOLE: OpenFile = OpenFile {
    ops: &SERIAL_OPS,
    offset: 0,
    private: 0,
};

pub fn console_file() -> *mut OpenFile {
    return &raw mut CONSOLE;
}
