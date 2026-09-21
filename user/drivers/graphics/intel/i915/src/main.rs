#![no_std]
#![no_main]

use libsys::{
    OP_PCI_FIND, SVC_PCI, log, map_mmio, mbox_call, mbox_connect, print, print_hex,
    set_msi_waker, spawn_thread, sys_sleep, sys_wakeup,
};
use compat_libc as _;

const DRIVER_LOG_TO_SERIAL: bool = true;

#[unsafe(no_mangle)]
pub extern "C" fn os_alloc_dma(pages: u64, phys_out: *mut u64) -> u64 {
    let (virt, phys) = libsys::alloc_dma(pages);
    unsafe {
        *phys_out = phys;
    }
    return virt;
}

#[unsafe(no_mangle)]
pub extern "C" fn os_map_mmio(phys: u64, pages: u64) -> u64 {
    map_mmio(phys, pages)
}

#[unsafe(no_mangle)]
pub extern "C" fn os_sleep(ident: u64, timeout_ns: u64) -> u64 {
    sys_sleep(ident, timeout_ns)
}

#[unsafe(no_mangle)]
pub extern "C" fn os_sleep_prepare(ident: u64, timeout_ns: u64) -> u64 {
    unsafe { libsys::syscall(32, ident, timeout_ns, 0, 0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn os_sleep_commit(handle: u64) -> u64 {
    unsafe { libsys::syscall(33, handle, 0, 0, 0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn os_wakeup(ident: u64) {
    sys_wakeup(ident);
}

#[unsafe(no_mangle)]
pub extern "C" fn os_msi_take() -> u64 {
    unsafe { libsys::syscall(30, 0, 0, 0, 0) }
}

#[unsafe(no_mangle)]
pub extern "C" fn os_print(s: *const u8) {
    unsafe {
        let mut len = 0usize;
        while *s.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(s, len);
        if let Ok(text) = core::str::from_utf8(slice) {
            if DRIVER_LOG_TO_SERIAL {
                print(text);
            } else {
                log(text);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn os_console_print(s: *const u8) {
    unsafe {
        let mut len = 0usize;
        while *s.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(s, len);
        if let Ok(text) = core::str::from_utf8(slice) {
            print(text);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn os_load_file(path: *const u8, out: *mut u8, max: usize) -> i64 {
    unsafe {
        let mut len = 0usize;
        while *path.add(len) != 0 {
            len += 1;
        }
        let path_slice = core::slice::from_raw_parts(path, len);
        let fd = libsys::open(path_slice);
        if fd < 0 {
            return -1;
        }
        let buf = core::slice::from_raw_parts_mut(out, max);
        let n = libsys::read(fd, buf);
        libsys::close(fd);
        n as i64
    }
}

unsafe extern "C" {
    fn i915_shim_probe(
        bus: u32,
        device: u32,
        function: u32,
        vendor_id: u32,
        product_id: u32,
        pci_class: u32,
        pci_subclass: u32,
        pci_progif: u32,
    ) -> i32;
    fn i915_shim_fb_info(
        width: *mut u32,
        height: *mut u32,
        depth: *mut u32,
        stride: *mut u32,
    ) -> i32;
    fn i915_shim_bounce_ready() -> i32;
    fn i915_shim_bounce_frame(
        old_x: u32,
        old_y: u32,
        new_x: u32,
        new_y: u32,
        size: u32,
        color: u32,
    ) -> i32;
    fn i915_worker_thread_entry() -> !;
    fn i915_irq_thread_entry() -> !;
    fn i915_shim_set_ram_pages(npages: u64);
}

extern "C" fn worker_trampoline() -> ! {
    unsafe { i915_worker_thread_entry() }
}

extern "C" fn irq_trampoline() -> ! {
    unsafe { i915_irq_thread_entry() }
}

extern "C" fn animation_trampoline() -> ! {
    log("i915: GPU_BOUNCE thread entered\n");
    while unsafe { i915_shim_bounce_ready() } == 0 {
        sys_sleep(0x1915_424f_554e_4345, 10_000_000);
    }
    log("i915: GPU_BOUNCE thread saw ready\n");

    let mut width = 0u32;
    let mut height = 0u32;
    let mut depth = 0u32;
    let mut stride = 0u32;
    let fb_ret = unsafe {
        i915_shim_fb_info(&mut width, &mut height, &mut depth, &mut stride)
    };
    if fb_ret != 0 {
        print("i915: GPU bounce framebuffer failed ret=");
        print_hex(fb_ret as u32);
        print("\n");
        loop {}
    }
    log("i915: GPU_BOUNCE framebuffer ready\n");

    let size = 128u32;
    let mut x = 32u32;
    let mut y = 32u32;
    let mut dx = 7i32;
    let mut dy = 5i32;
    let mut frame = 0u32;
    loop {
        let old_x = x;
        let old_y = y;
        let next_x = x as i32 + dx;
        let next_y = y as i32 + dy;

        if next_x <= 0 || next_x + size as i32 >= width as i32 {
            dx = -dx;
        }
        if next_y <= 0 || next_y + size as i32 >= height as i32 {
            dy = -dy;
        }

        x = (x as i32 + dx) as u32;
        y = (y as i32 + dy) as u32;
        let color = match (frame / 180) % 3 {
            0 => 0x0000d8ff,
            1 => 0x00ff3b80,
            _ => 0x0099ff55,
        };
        if frame == 0 {
            log("i915: GPU_BOUNCE first frame enter\n");
        }
        let ret = unsafe {
            i915_shim_bounce_frame(old_x, old_y, x, y, size, color)
        };
        if frame == 0 {
            log("i915: GPU_BOUNCE first frame returned\n");
        }
        if ret != 0 {
            print("i915: GPU bounce frame failed ret=");
            print_hex(ret as u32);
            print("\n");
            loop {}
        }
        frame = frame.wrapping_add(1);
        sys_sleep(0x1915_424f_554e_4345, 16_666_667);
    }
}

libsys::entry!(main);

unsafe extern "C" fn main() -> ! {
    spawn_thread(worker_trampoline, 3, 1);
    if spawn_thread(irq_trampoline, 3, 1) < 0 {
        log("i915: IRQ thread spawn failed\n");
    }
    let animation_pid = spawn_thread(animation_trampoline, 3, 1);
    if animation_pid < 0 {
        log("i915: GPU_BOUNCE thread spawn failed\n");
    } else {
        log("i915: GPU_BOUNCE thread spawned\n");
    }
    set_msi_waker(0x1915000000000001);

    unsafe {
        i915_shim_set_ram_pages(libsys::total_ram_pages());
    }

    let idx = mbox_connect(SVC_PCI);
    let req = [OP_PCI_FIND, 0x03, 0x00];
    let mut out = [0u8; 20];
    mbox_call(idx, &req, &mut out);

    if out[0] == 0 {
        print("i915: no Intel display controller found\n");
        loop {}
    }

    let bus: u32 = 0;
    let device = out[2] as u32;
    let function: u32 = 0;
    let vendor = u16::from_le_bytes([out[4], out[5]]) as u32;
    let product = u16::from_le_bytes([out[6], out[7]]) as u32;
    let class = out[8] as u32;
    let subclass = out[9] as u32;
    let progif = out[10] as u32;

    print("i915: found device vendor=");
    print_hex(vendor);
    print(" product=");
    print_hex(product);
    print("\n");

    let ret = unsafe {
        i915_shim_probe(bus, device, function, vendor, product, class, subclass, progif)
    };

    if ret != 0 {
        print("i915: probe did not match/attach\n");
        loop {}
    }
    print("i915: probe returned, attach path ran\n");

    loop {
        sys_sleep(0x1915_4d41_494e_0001, 1_000_000_000);
    }
}
