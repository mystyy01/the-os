#![no_std]
#![no_main]

use libsys::{OP_PCI_FIND, SVC_PCI, map_mmio, mbox_call, mbox_connect, print, print_hex};
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
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
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
    } else {
        print("i915: probe returned, attach path ran\n");
    }

    loop {}
}
