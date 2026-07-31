#![no_std]
#![no_main]

mod fake_libc_thing;

use libsys::{
    OP_BIND, OP_BWRITE, OP_CLOSE, OP_OPEN, OP_READ, OP_SYNC, OP_WRITE, SVC_ATA, SVC_FS, mbox_call,
    mbox_connect, print, print_hex, register, serve, vfs_bind, vfs_resolve,
};

// needs to match lwext4
#[repr(C)]
struct Ext4BlockdevIface {
    open: Option<unsafe extern "C" fn(*mut Ext4Blockdev) -> i32>,
    bread: Option<unsafe extern "C" fn(*mut Ext4Blockdev, *mut u8, u64, u32) -> i32>,
    bwrite: Option<unsafe extern "C" fn(*mut Ext4Blockdev, *const u8, u64, u32) -> i32>,
    close: Option<unsafe extern "C" fn(*mut Ext4Blockdev) -> i32>,
    lock: Option<unsafe extern "C" fn(*mut Ext4Blockdev) -> i32>,
    unlock: Option<unsafe extern "C" fn(*mut Ext4Blockdev) -> i32>,
    ph_bsize: u32,
    ph_bcnt: u64,
    ph_bbuf: *mut u8,
    ph_refctr: u32,
    bread_ctr: u32,
    bwrite_ctr: u32,
    p_user: *mut u8,
}
// and this needs to match too
#[repr(C)]
struct Ext4Blockdev {
    bdif: *mut Ext4BlockdevIface,
    part_offset: u64,
    part_size: u64,
    bc: *mut u8,
    lg_bsize: u32,
    lg_bcnt: u64,
    cache_write_back: u32,
    fs: *mut u8,
    journal: *mut u8,
}

unsafe extern "C" fn bread(
    _bdev: *mut Ext4Blockdev,
    buf: *mut u8,
    blk_id: u64,
    blk_cnt: u32,
) -> i32 {
    let mut req = [0u8; 8];
    req[0] = OP_READ;
    req[1..5].copy_from_slice(&(blk_id as u32).to_le_bytes());
    req[5] = blk_cnt as u8;
    let mut reply = [0u8; 4096];
    let n = mbox_call(ATA_CONN, &req[..6], &mut reply);
    let len = (blk_cnt as usize * 512).min(n);
    core::ptr::copy_nonoverlapping(reply.as_ptr(), buf, len);
    0
}

unsafe extern "C" fn bd_open(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}
unsafe extern "C" fn bd_close(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}

static mut ATA_CONN: usize = 0;
static mut IFACE: Ext4BlockdevIface = Ext4BlockdevIface {
    open: None,
    bread: None,
    bwrite: None,
    close: None,
    lock: None,
    unlock: None,
    ph_bsize: 0,
    ph_bcnt: 0,
    ph_bbuf: core::ptr::null_mut(),
    ph_refctr: 0,
    bread_ctr: 0,
    bwrite_ctr: 0,
    p_user: core::ptr::null_mut(),
};

static mut BLOCKDEV: Ext4Blockdev = Ext4Blockdev {
    bdif: core::ptr::null_mut(),
    part_offset: 0,
    part_size: 0,
    bc: core::ptr::null_mut(),
    lg_bsize: 0,
    lg_bcnt: 0,
    cache_write_back: 0,
    fs: core::ptr::null_mut(),
    journal: core::ptr::null_mut(),
};

#[repr(C)]
#[derive(Clone, Copy)]
struct Ext4File {
    mp: *mut u8,
    inode: u32,
    flags: u32,
    fsize: u64,
    fpos: u64,
}

unsafe extern "C" {
    fn ext4_device_register(bd: *mut Ext4Blockdev, dev_name: *const u8) -> i32;
    fn ext4_mount(dev_name: *const u8, mount_point: *const u8, read_write: i32) -> i32;
    fn ext4_fopen(file: *mut Ext4File, path: *const u8, flags: *const u8) -> i32;
    fn ext4_fread(file: *mut Ext4File, buf: *mut u8, size: usize, rcnt: *mut usize) -> i32;
    fn ext4_fwrite(file: *mut Ext4File, buf: *const u8, size: usize, wcnt: *mut usize) -> i32;
    fn ext4_fclose(file: *mut Ext4File) -> i32;
}
const MAX_FILES: usize = 128;
static mut FILES: [Ext4File; MAX_FILES] = [Ext4File {
    mp: core::ptr::null_mut(),
    inode: 0,
    flags: 0,
    fsize: 0,
    fpos: 0,
}; MAX_FILES];
static mut USED: [bool; MAX_FILES] = [false; MAX_FILES];

fn on_open(req: &[u8], reply: &mut [u8]) -> usize {
    let mut slot = -1i32;
    unsafe {
        for i in 0..MAX_FILES {
            if !USED[i] {
                slot = i as i32;
                break;
            }
        }
    }
    if slot < 0 {
        reply[..4].copy_from_slice(&(-1i32).to_le_bytes());
        return 4;
    }
    let flags = req[1];
    let mode: &[u8] = match flags {
        2 => b"w\0",
        1 => b"a\0",
        _ => b"r\0",
    };
    let mut path = [0u8; 256];
    let n = core::cmp::min(req.len() - 2, 254);
    path[..n].copy_from_slice(&req[2..2 + n]);
    let rc = unsafe {
        ext4_fopen(&raw mut FILES[slot as usize], path.as_ptr(), mode.as_ptr())
    };
    let handle = if rc == 0 {
        unsafe {
            USED[slot as usize] = true;
        }
        slot
    } else {
        -1
    };
    reply[..4].copy_from_slice(&handle.to_le_bytes());
    4
}

fn on_read(req: &[u8], reply: &mut [u8]) -> usize {
    let handle = i32::from_le_bytes([req[1], req[2], req[3], req[4]]);
    let maxlen = u32::from_le_bytes([req[5], req[6], req[7], req[8]]) as usize;
    if handle < 0 || handle as usize >= MAX_FILES || unsafe { !USED[handle as usize] } {
        return 0;
    }
    let want = core::cmp::min(maxlen, reply.len());
    let mut got: usize = 0;
    unsafe {
        ext4_fread(
            &raw mut FILES[handle as usize],
            reply.as_mut_ptr(),
            want,
            &mut got,
        );
    }
    got
}

fn on_write(req: &[u8], reply: &mut [u8]) -> usize {
    let handle = i32::from_le_bytes([req[1], req[2], req[3], req[4]]);
    if handle < 0 || handle as usize >= MAX_FILES || unsafe { !USED[handle as usize] } {
        reply[..4].copy_from_slice(&(-1i32).to_le_bytes());
        return 4;
    }
    let data = &req[5..];
    let mut got: usize = 0;
    unsafe {
        ext4_fwrite(
            &raw mut FILES[handle as usize],
            data.as_ptr(),
            data.len(),
            &mut got,
        );
    }
    reply[..4].copy_from_slice(&(got as i32).to_le_bytes());
    4
}

fn on_close(req: &[u8], reply: &mut [u8]) -> usize {
    let handle = i32::from_le_bytes([req[1], req[2], req[3], req[4]]);
    if handle >= 0 && (handle as usize) < MAX_FILES && unsafe { USED[handle as usize] } {
        unsafe {
            ext4_fclose(&raw mut FILES[handle as usize]);
            USED[handle as usize] = false;
            let sync = [OP_SYNC];
            let mut r = [0u8; 4];
            mbox_call(ATA_CONN, &sync, &mut r);
        }
    }
    reply[..4].copy_from_slice(&0i32.to_le_bytes());
    4
}

static mut PH_BBUF: [u8; 512] = [0; 512];

unsafe extern "C" fn bd_lock(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}
unsafe extern "C" fn bd_unlock(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}
static mut BWRITE_BUF: [u8; 6 + 8 * 512] = [0u8; 6 + 8 * 512];

unsafe extern "C" fn bd_bwrite(
    _bdev: *mut Ext4Blockdev,
    buf: *const u8,
    blk_id: u64,
    blk_cnt: u32,
) -> i32 {
    let req = &raw mut BWRITE_BUF as *mut u8;
    let mut done: u32 = 0;
    while done < blk_cnt {
        let c = core::cmp::min(blk_cnt - done, 7);
        let bytes = (c * 512) as usize;
        unsafe {
            *req.add(0) = OP_BWRITE;
            core::ptr::copy_nonoverlapping(
                ((blk_id as u32) + done).to_le_bytes().as_ptr(),
                req.add(1),
                4,
            );
            *req.add(5) = c as u8;
            core::ptr::copy_nonoverlapping(buf.add((done * 512) as usize), req.add(6), bytes);
            let slice = core::slice::from_raw_parts(req, 6 + bytes);
            let mut reply = [0u8; 4];
            mbox_call(ATA_CONN, slice, &mut reply);
        }
        done += c;
    }
    0
}
fn read_lba(lba: u32, out: &mut [u8; 512]) {
    let mut req = [0u8; 6];
    req[0] = OP_READ;
    req[1..5].copy_from_slice(&lba.to_le_bytes());
    req[5] = 1;
    let mut reply = [0u8; 4096];
    unsafe {
        mbox_call(ATA_CONN, &req[..6], &mut reply);
    }
    out.copy_from_slice(&reply[..512]);
}

fn is_ext4_at(lba: u32) -> bool {
    let mut b = [0u8; 512];
    read_lba(lba + 2, &mut b);
    b[56] == 0x53 && b[57] == 0xEF
}

fn detect_partition() -> (u64, u64) {
    let mut hdr = [0u8; 512];
    read_lba(1, &mut hdr);
    if &hdr[..8] == b"EFI PART" {
        let entry_lba = u32::from_le_bytes([hdr[72], hdr[73], hdr[74], hdr[75]]);
        let num = u32::from_le_bytes([hdr[80], hdr[81], hdr[82], hdr[83]]).min(128);
        let esize = u32::from_le_bytes([hdr[84], hdr[85], hdr[86], hdr[87]]).max(1);
        let per_block = (512 / esize).max(1);
        let mut idx = 0u32;
        while idx < num {
            let mut eb = [0u8; 512];
            read_lba(entry_lba + idx / per_block, &mut eb);
            let o = ((idx % per_block) * esize) as usize;
            let first =
                u32::from_le_bytes([eb[o + 32], eb[o + 33], eb[o + 34], eb[o + 35]]);
            let last = u32::from_le_bytes([eb[o + 40], eb[o + 41], eb[o + 42], eb[o + 43]]);
            if first != 0 && is_ext4_at(first) {
                return ((first as u64) * 512, (last - first + 1) as u64);
            }
            idx += 1;
        }
    }
    (0, 131072)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let ata = loop {
        let r = vfs_resolve("/dev/ata0".as_bytes());
        if r != 0 {
            break r;
        }
        unsafe {
            libsys::syscall(9, 0, 0, 0, 0);
        } // yield
    };
    let ata_conn = mbox_connect(ata);
    unsafe {
        ATA_CONN = ata_conn;

        IFACE.bread = Some(bread);
        IFACE.open = Some(bd_open);
        IFACE.close = Some(bd_close);
        IFACE.lock = Some(bd_lock);
        IFACE.unlock = Some(bd_unlock);
        IFACE.bwrite = Some(bd_bwrite);
        let (poff, pblocks) = detect_partition();
        print("FS: ext4 part_offset=");
        print_hex((poff >> 32) as u32);
        print_hex(poff as u32);
        print("\n");

        IFACE.ph_bbuf = &raw mut PH_BBUF as *mut u8;
        IFACE.ph_bsize = 512;
        IFACE.ph_bcnt = poff / 512 + pblocks;
        BLOCKDEV.part_offset = poff;
        BLOCKDEV.part_size = pblocks * 512;

        BLOCKDEV.bdif = &raw mut IFACE;
        BLOCKDEV.lg_bsize = 512;
        BLOCKDEV.lg_bcnt = pblocks;

        let dr = ext4_device_register(&raw mut BLOCKDEV, b"ata0\0".as_ptr());
        if dr == 0 {
            print("FS: dev ok\n");
        } else {
            print("FS: dev FAIL\n");
        }
        let mr = ext4_mount(b"ata0\0".as_ptr(), b"/\0".as_ptr(), 0);
        if mr == 0 {
            print("FS: mount ok\n");
        } else {
            print("FS: mount FAIL\n");
        }
    }

    register(OP_READ, on_read);
    register(OP_OPEN, on_open);
    register(OP_WRITE, on_write);
    register(OP_CLOSE, on_close);

    vfs_bind("/".as_bytes(), SVC_FS);

    let svc_id = vfs_resolve(b"/run/init");
    let idx = mbox_connect(svc_id);
    let mut req = [0u8; 1];
    req[0] = OP_BIND;
    let mut out = [0u8; 4];
    mbox_call(idx, &req, &mut out);

    serve(SVC_FS);

    loop {}
}
