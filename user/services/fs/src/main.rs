#![no_std]
#![no_main]

mod fake_libc_thing;

use libsys::{
    self, IPCMessage, OP_OPEN, OP_READ, OP_WRITE, call, connect, get_self_pid, print, register,
    serve, vfs_bind, vfs_resolve,
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
}

unsafe extern "C" fn bread(
    bdev: *mut Ext4Blockdev,
    buf: *mut u8,
    blk_id: u64,
    blk_cnt: u32,
) -> i32 {
    let mut data = [0u8; 4096];
    data[0] = OP_READ;
    data[1..5].copy_from_slice(&(blk_id as u32).to_le_bytes());
    data[5] = blk_cnt as u8;
    let msg = IPCMessage { data, len: 6 };
    let reply = call(ATA_IPCD, msg);
    let len = blk_cnt as usize * 512;
    core::ptr::copy_nonoverlapping(reply.data.as_ptr(), buf, len);
    return 0;
}

unsafe extern "C" fn bd_open(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}
unsafe extern "C" fn bd_close(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}

static mut ATA_IPCD: i32 = -1;
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

fn on_open(req: &IPCMessage, reply: &mut IPCMessage) {
    // req: [OP_OPEN][path bytes...], path NUL-terminated by us
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
        reply.data[..4].copy_from_slice(&(-1i32).to_le_bytes());
        reply.len = 4;
        return;
    }

    // build a NUL-terminated C path from the request payload
    let mut path = [0u8; 256];
    let n = core::cmp::min(req.len - 1, 254);
    path[..n].copy_from_slice(&req.data[1..1 + n]);
    // path[n] already 0

    let rc = unsafe {
        ext4_fopen(
            &raw mut FILES[slot as usize],
            path.as_ptr(),
            b"r\0".as_ptr(),
        )
    };
    let handle = if rc == 0 {
        unsafe {
            USED[slot as usize] = true;
        }
        slot
    } else {
        -1
    };
    reply.data[..4].copy_from_slice(&handle.to_le_bytes());
    reply.len = 4;
}

fn on_read(req: &IPCMessage, reply: &mut IPCMessage) {
    // req: [OP_READ][handle i32][maxlen u32]
    let handle = i32::from_le_bytes([req.data[1], req.data[2], req.data[3], req.data[4]]);
    let maxlen = u32::from_le_bytes([req.data[5], req.data[6], req.data[7], req.data[8]]) as usize;
    if handle < 0 || handle as usize >= MAX_FILES || unsafe { !USED[handle as usize] } {
        reply.len = 0;
        return;
    }
    let want = core::cmp::min(maxlen, reply.data.len());
    let mut got: usize = 0;
    unsafe {
        ext4_fread(
            &raw mut FILES[handle as usize],
            reply.data.as_mut_ptr(),
            want,
            &mut got,
        );
    }
    reply.len = got;
}

fn on_write(req: &IPCMessage, reply: &mut IPCMessage) {
    let n = req.len.saturating_sub(1) as i32;
    reply.data[..4].copy_from_slice(&n.to_le_bytes());
    reply.len = 4;
}

static mut PH_BBUF: [u8; 512] = [0; 512];

unsafe extern "C" fn bd_lock(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}
unsafe extern "C" fn bd_unlock(_bdev: *mut Ext4Blockdev) -> i32 {
    0
}
unsafe extern "C" fn bd_bwrite(
    _bdev: *mut Ext4Blockdev,
    _buf: *const u8,
    _blk_id: u64,
    _blk_cnt: u32,
) -> i32 {
    0
}
#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let mut ata_pid = vfs_resolve("/dev/ata0".as_bytes());
    while ata_pid < 0 {
        ata_pid = vfs_resolve("/dev/ata0".as_bytes());
    }
    let ata_ipcd = connect(ata_pid);
    if ata_ipcd < 0 {
        print("FS: ata BAD\n");
    } else {
        print("FS: ata ok\n");
    }
    unsafe {
        ATA_IPCD = ata_ipcd;

        IFACE.bread = Some(bread);
        IFACE.open = Some(bd_open);
        IFACE.close = Some(bd_close);
        IFACE.lock = Some(bd_lock);
        IFACE.unlock = Some(bd_unlock);
        IFACE.bwrite = Some(bd_bwrite);
        IFACE.ph_bbuf = &raw mut PH_BBUF as *mut u8;
        IFACE.ph_bsize = 512;
        IFACE.ph_bcnt = 131072;
        BLOCKDEV.part_offset = 0;
        BLOCKDEV.part_size = 512 * 131072;

        BLOCKDEV.bdif = &raw mut IFACE;
        BLOCKDEV.lg_bsize = 512;
        BLOCKDEV.lg_bcnt = 131072;

        let dr = ext4_device_register(&raw mut BLOCKDEV, b"ata0\0".as_ptr());
        if dr == 0 {
            print("FS: dev ok\n");
        } else {
            print("FS: dev FAIL\n");
        }
        let mr = ext4_mount(b"ata0\0".as_ptr(), b"/\0".as_ptr(), 1);
        if mr == 0 {
            print("FS: mount ok\n");
        } else {
            print("FS: mount FAIL\n");
        }
    }
    register(OP_READ, on_read);
    register(OP_OPEN, on_open);
    register(OP_WRITE, on_write);

    vfs_bind("/".as_bytes(), get_self_pid());

    serve();
}
