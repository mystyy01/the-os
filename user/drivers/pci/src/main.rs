#![no_std]
#![no_main]

use libsys::{OP_PCI_CFG_READ32, OP_PCI_FIND, SVC_PCI, inl, outl, register, serve};

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
    for device in 0..32u8 {
        let (vendor, dev_id) = vendor_device(0, device, 0);
        if vendor == 0xFFFF {
            continue;
        }
        let (class, subclass, prog_if) = class_info(0, device, 0);
        if class != want_class {
            continue;
        }
        if want_subclass != 0xFF && subclass != want_subclass {
            continue;
        }
        let bar0 = bar(0, device, 0, 0);
        let bar1 = bar(0, device, 0, 1);
        reply[0] = 1;
        reply[2] = device;
        reply[4..6].copy_from_slice(&vendor.to_le_bytes());
        reply[6..8].copy_from_slice(&dev_id.to_le_bytes());
        reply[8] = class;
        reply[9] = subclass;
        reply[10] = prog_if;
        reply[12..16].copy_from_slice(&bar0.to_le_bytes());
        reply[16..20].copy_from_slice(&bar1.to_le_bytes());
        return 20;
    }
    reply[0] = 0;
    1
}

fn on_cfg_read32(req: &[u8], reply: &mut [u8]) -> usize {
    let bus = req[1];
    let device = req[2];
    let function = req[3];
    let offset = req[4];
    let val = config_read32(bus, device, function, offset);
    reply[0..4].copy_from_slice(&val.to_le_bytes());
    4
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start() -> ! {
    register(OP_PCI_FIND, on_find);
    register(OP_PCI_CFG_READ32, on_cfg_read32);
    serve(SVC_PCI);
    loop {}
}
