#![no_std]
#![no_main]

use libsys::{
    OP_BWRITE, OP_READ, OP_SYNC, SVC_ATA, map_mmio, module_info, print, print_hex, register, serve,
    vfs_bind,
};

const SECTOR: usize = 512;
const MAX_SECTORS: u8 = 8;

static mut BASE: u64 = 0;
static mut LEN: u64 = 0;

fn in_range(off: u64, n: u64) -> bool {
    let len = unsafe { LEN };
    off < len && n <= len - off
}

fn on_read(req: &[u8], reply: &mut [u8]) -> usize {
    let lba = u32::from_le_bytes([req[1], req[2], req[3], req[4]]);
    let mut count = req[5];
    if count > MAX_SECTORS {
        count = MAX_SECTORS;
    }
    let n = count as usize * SECTOR;
    let off = lba as u64 * SECTOR as u64;
    if !in_range(off, n as u64) {
        print("RAMDISK: rd OOR lba=");
        print_hex(lba);
        print("\n");
        return 0;
    }
    unsafe {
        core::ptr::copy_nonoverlapping((BASE + off) as *const u8, reply.as_mut_ptr(), n);
    }
    n
}

fn on_bwrite(req: &[u8], reply: &mut [u8]) -> usize {
    let lba = u32::from_le_bytes([req[1], req[2], req[3], req[4]]);
    let mut count = req[5];
    if count > MAX_SECTORS {
        count = MAX_SECTORS;
    }
    let n = count as usize * SECTOR;
    let off = lba as u64 * SECTOR as u64;
    if !in_range(off, n as u64) {
        print("RAMDISK: wr OOR lba=");
        print_hex(lba);
        print("\n");
        reply[..4].copy_from_slice(&(-1i32).to_le_bytes());
        return 4;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(req.as_ptr().add(6), (BASE + off) as *mut u8, n);
    }
    reply[..4].copy_from_slice(&0i32.to_le_bytes());
    4
}

fn on_sync(_req: &[u8], reply: &mut [u8]) -> usize {
    reply[..4].copy_from_slice(&0i32.to_le_bytes());
    4
}

libsys::entry!(main);

unsafe extern "C" fn main() -> ! {
    let (phys, len) = module_info(0);
    if phys == 0 || len == 0 {
        print("RAMDISK: no boot module\n");
        panic!();
    }

    let page_off = phys & 0xFFF;
    let pages = (page_off + len + 0xFFF) / 0x1000;
    let virt = map_mmio(phys, pages);
    if virt == 0 {
        print("RAMDISK: map failed\n");
        panic!();
    }

    unsafe {
        BASE = virt + page_off;
        LEN = len;
    }

    print("RAMDISK: phys=");
    print_hex(phys as u32);
    print(" len=");
    print_hex(len as u32);
    print(" sectors=");
    print_hex((len / SECTOR as u64) as u32);
    print("\n");

    vfs_bind("/dev/ata0".as_bytes(), SVC_ATA);

    register(OP_READ, on_read);
    register(OP_BWRITE, on_bwrite);
    register(OP_SYNC, on_sync);

    serve(SVC_ATA);
    loop {}
}
