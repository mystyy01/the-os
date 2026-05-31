use crate::{
    msr::{rdmsr, wrmsr},
    serial,
};

unsafe extern "C" {
    fn syscall_entry();
}

pub unsafe fn init() {
    unsafe {
        // efer
        let efer_num = 0xC0000080u32;
        let efer = rdmsr(efer_num);
        wrmsr(efer_num, efer | 1);

        // star
        wrmsr(0xC0000081u32, (0x0008u64 << 32) | (0x0010u64 << 48));

        // lstar
        wrmsr(0xC0000082u32, syscall_entry as u64);

        // sfmask
        wrmsr(0xC0000084u32, 1u64 << 9);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(nr: u64) -> u64 {
    match nr {
        0 => {
            serial::write_str("Syscall 0!\n");
            return 0;
        }
        _ => u64::MAX,
    }
}
