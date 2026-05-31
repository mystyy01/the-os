use core::cmp::min;
use core::ptr::copy_nonoverlapping;

use crate::{pmm, vmm};

macro_rules! read_field {
    ($ptr:expr, $field:ident) => {
        core::ptr::read_unaligned(core::ptr::addr_of!((*$ptr).$field))
    };
}

const OSABI_NUM: u8 = 0xAE;

#[repr(C, packed)]
struct ELFHeader {
    ident: [u8; 16],
    elf_type: u16,
    machine: u16,
    version: u32,
    entry: u64,
    phoff: u64,
    shoff: u64,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

#[repr(C, packed)]
struct ProgramHeader {
    seq_type: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    paddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

pub unsafe fn load(data: *const u8, size: usize, pml4: *mut u64) -> Option<u64> {
    unsafe {
        if size < 64 {
            return None;
        }
        let header = data as *const ELFHeader;
        let ident = read_field!(header, ident);

        if ident[0..4] != [0x7F, b'E', b'L', b'F'] {
            return None;
        }
        if ident[4] != 2 {
            return None;
        }
        if ident[7] != OSABI_NUM {
            return None;
        }

        let entry = read_field!(header, entry);
        let phoff = read_field!(header, phoff);
        let phnum = read_field!(header, phnum);
        let phentsize = read_field!(header, phentsize);

        if phoff + (phnum as u64 * phentsize as u64) > size as u64 {
            return None;
        }
        for i in 0..phnum {
            let pgheader =
                data.add(phoff as usize + i as usize * phentsize as usize) as *const ProgramHeader;
            if read_field!(pgheader, seq_type) != 1 {
                continue;
            }
            let offset = read_field!(pgheader, offset);
            let vaddr = read_field!(pgheader, vaddr);
            let filesz = read_field!(pgheader, filesz);
            let memsz = read_field!(pgheader, memsz);

            let delta = vaddr & 0xFFF;
            let vaddr_aligned = vaddr & !0xFFF;

            let page_count = (delta + memsz + 0xFFF) / 0x1000;
            for page_num in 0..page_count {
                let phys_page = pmm::alloc_pages(0);
                if phys_page.is_null() {
                    return None;
                }
                core::ptr::write_bytes(phys_page, 0, 0x1000);

                let page_start = page_num * 0x1000;
                let copy_start = page_start.max(delta);
                let copy_end = (delta + filesz).min(page_start + 0x1000);

                if copy_end > copy_start {
                    let dst_off = copy_start - page_start;
                    let src_rel = copy_start - delta;
                    let len = copy_end - copy_start;
                    copy_nonoverlapping(
                        data.add(offset as usize + src_rel as usize),
                        phys_page.add(dst_off as usize),
                        len as usize,
                    );
                }

                vmm::map_page(
                    pml4,
                    vaddr_aligned + page_num as u64 * 0x1000,
                    phys_page as u64,
                    0x07,
                );
            }
        }
        return Some(entry);
    }
}
