use crate::io::{inl, outl};

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

fn config_read32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address: u32 = (1 << 31)
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC);
    outl(CONFIG_ADDRESS, address);
    inl(CONFIG_DATA)
}

pub fn vendor_device(bus: u8, device: u8, function: u8) -> (u16, u16) {
    let val = config_read32(bus, device, function, 0x00);
    ((val & 0xFFFF) as u16, (val >> 16) as u16)
}
