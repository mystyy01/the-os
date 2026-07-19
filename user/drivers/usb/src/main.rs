#![no_std]
#![no_main]

use libsys::{OP_PCI_FIND, SVC_PCI, map_mmio, mbox_call, mbox_connect, print, print_hex};

fn mmio_read32(base: u64, off: u64) -> u32 {
    unsafe { core::ptr::read_volatile((base + off) as *const u32) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    let idx = mbox_connect(SVC_PCI);
    let req = [OP_PCI_FIND, 0x0C, 0x03];
    let mut out = [0u8; 20];
    mbox_call(idx, &req, &mut out);

    if out[0] == 0 {
        print("USB: no xHCI controller found\n");
        loop {}
    }

    let bar0 = u32::from_le_bytes([out[12], out[13], out[14], out[15]]);
    let bar1 = u32::from_le_bytes([out[16], out[17], out[18], out[19]]);
    let phys = ((bar0 as u64) & 0xFFFF_FFF0) | ((bar1 as u64) << 32);

    print("USB: xHCI BAR phys=");
    print_hex((phys >> 32) as u32);
    print_hex(phys as u32);
    print("\n");

    let vbase = map_mmio(phys, 4);

    let cap_len = mmio_read32(vbase, 0x00) & 0xFF;
    let hcsparams1 = mmio_read32(vbase, 0x04);
    let hccparams1 = mmio_read32(vbase, 0x10);

    print("USB: caplen=");
    print_hex(cap_len);
    print(" hcsparams1=");
    print_hex(hcsparams1);
    print(" hccparams1=");
    print_hex(hccparams1);
    print("\n");

    let max_slots = hcsparams1 & 0xFF;
    let max_ports = (hcsparams1 >> 24) & 0xFF;
    print("USB: max_slots=");
    print_hex(max_slots);
    print(" max_ports=");
    print_hex(max_ports);
    print("\n");

    loop {}
}
