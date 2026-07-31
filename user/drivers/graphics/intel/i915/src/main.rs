#![no_std]
#![no_main]

use libsys::{
    OP_PCI_FIND, SVC_PCI, map_mmio, mbox_call, mbox_connect, print, print_hex,
    spawn_thread, sys_sleep, sys_wakeup,
    set_msi_waker,
};
use compat_libc as _;

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
    fn i915_shim_draw_square(x: u32, y: u32, w: u32, h: u32, rgb: u32) -> i32;
    fn i915_worker_thread_entry() -> !;
    fn i915_shim_set_ram_pages(npages: u64);
}

extern "C" fn worker_trampoline() -> ! {
    unsafe { i915_worker_thread_entry() }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    spawn_thread(worker_trampoline, 3, 1);
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

    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut depth: u32 = 0;
    let mut stride: u32 = 0;
    let fb_ret = unsafe {
        i915_shim_fb_info(&mut width, &mut height, &mut depth, &mut stride)
    };

    if fb_ret != 0 {
        print("i915: no framebuffer (ri_bits is NULL) - fb_ret=");
        print_hex(fb_ret as u32);
        print("\n");
        loop {}
    }

    print("i915: fb width=");
    print_hex(width);
    print(" height=");
    print_hex(height);
    print(" depth=");
    print_hex(depth);
    print(" stride=");
    print_hex(stride);
    print("\n");

    print("i915: filling framebuffer (color cycle)\n");

    let mut c: u32 = 0;
    loop {
        let color = match c % 3 {
            0 => 0x00FF0000u32,
            1 => 0x0000FF00u32,
            _ => 0x000000FFu32,
        };
        unsafe {
            i915_shim_draw_square(0, 0, width, height, color);
        }
        c = c.wrapping_add(1);
        let mut d: u64 = 0;
        while d < 80_000_000 {
            core::hint::spin_loop();
            d += 1;
        }
    }
}
