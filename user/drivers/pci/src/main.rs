#![no_std]
#![no_main]

use libsys::{OP_PCI_FIND, SVC_PCI, inl, outl, register, serve};

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

fn config_read32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address: u32 = (1 << 31)
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC);
    unsafe {
        outl(CONFIG_ADDRESS, address);
        inl(CONFIG_DATA)
    }
}

fn vendor_device(bus: u8, device: u8, function: u8) -> (u16, u16) {
    let val = config_read32(bus, device, function, 0x00);
    ((val & 0xFFFF) as u16, (val >> 16) as u16)
}

fn class_info(bus: u8, device: u8, function: u8) -> (u8, u8, u8) {
    let val = config_read32(bus, device, function, 0x08);
    ((val >> 24) as u8, (val >> 16) as u8, (val >> 8) as u8)
}

fn bar(bus: u8, device: u8, function: u8, n: u8) -> u32 {
    config_read32(bus, device, function, 0x10 + n * 4)
}

fn on_find(req: &[u8], reply: &mut [u8]) -> usize {
    let want_class = req[1];
    let want_subclass = req[2];
    let want_prog_if = if req.len() > 3 { req[3] } else { 0xFF };
    let want_index = if req.len() > 4 { req[4] } else { 0 };
    let mut seen = 0u8;
    for bus in 0u8..=255u8 {
        for device in 0..32u8 {
            for function in 0..8u8 {
                let (vendor, dev_id) = vendor_device(bus, device, function);
                if vendor == 0xFFFF {
                    continue;
                }
                let (class, subclass, prog_if) = class_info(bus, device, function);
                if class != want_class {
                    continue;
                }
                if want_subclass != 0xFF && subclass != want_subclass {
                    continue;
                }
                if want_prog_if != 0xFF && prog_if != want_prog_if {
                    continue;
                }
                if seen != want_index {
                    seen += 1;
                    continue;
                }
                let bar0 = bar(bus, device, function, 0);
                let bar1 = bar(bus, device, function, 1);
                reply[0] = 1;
                reply[1] = bus;
                reply[2] = device;
                reply[3] = function;
                reply[4..6].copy_from_slice(&vendor.to_le_bytes());
                reply[6..8].copy_from_slice(&dev_id.to_le_bytes());
                reply[8] = class;
                reply[9] = subclass;
                reply[10] = prog_if;
                reply[12..16].copy_from_slice(&bar0.to_le_bytes());
                reply[16..20].copy_from_slice(&bar1.to_le_bytes());
                return 20;
            }
        }
    }
    reply[0] = 0;
    1
}

libsys::entry!(main);

unsafe extern "C" fn main() -> ! {
    register(OP_PCI_FIND, on_find);
    serve(SVC_PCI);
    loop {}
}
