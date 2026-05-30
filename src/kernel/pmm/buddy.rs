use core::ptr::{null, null_mut};

use alloc::collections::btree_set;

use crate::pmm::MAX_ORDER;
use crate::pmm::PAGE_SIZE;

#[repr(C)]
struct FreeBlock {
    next: *mut FreeBlock,
}
unsafe extern "C" {
    static kernel_start: u8;
    static kernel_end: u8;
}

struct BuddyAllocator {
    free_lists: [*mut FreeBlock; 11],
    total_memory: u64,
    free_memory: u64,
}

static mut BUDDY: BuddyAllocator = BuddyAllocator {
    free_lists: [core::ptr::null_mut(); 11],
    total_memory: 0,
    free_memory: 0,
};

fn free_page(addr: u64, order: usize) -> () {
    unsafe {
        let block: *mut FreeBlock = addr as *mut FreeBlock;
        (*block).next = BUDDY.free_lists[order];
        BUDDY.free_lists[order] = block;
        BUDDY.free_memory += PAGE_SIZE << order;
    }
}

pub fn alloc_pages(order: usize) -> *mut u8 {
    unsafe {
        if !BUDDY.free_lists[order].is_null() {
            let free_block = BUDDY.free_lists[order];
            BUDDY.free_lists[order] = (*free_block).next;
            BUDDY.free_memory -= PAGE_SIZE << order;
            return free_block as *mut u8;
        }
        if order + 1 >= MAX_ORDER {
            return null_mut();
        }
        let block = alloc_pages(order + 1);
        if block.is_null() {
            return null_mut();
        }
        free_page(block as u64 + (PAGE_SIZE << order), order);
        return block;
    }
}

pub fn free_pages(order: usize, addr: u64) -> () {
    let mut current_addr = addr;
    let mut current_order = order;
    unsafe {
        loop {
            if current_order == MAX_ORDER - 1 {
                free_page(current_addr, current_order);
                break;
            }

            let buddy_addr = current_addr ^ (PAGE_SIZE << current_order);
            let mut prev: *mut *mut FreeBlock = &mut BUDDY.free_lists[current_order];
            let mut curr = BUDDY.free_lists[current_order];
            while !curr.is_null() {
                if curr as u64 == buddy_addr {
                    *prev = (*curr).next;
                    break;
                }
                prev = &mut (*curr).next;
                curr = (*curr).next;
            }
            if curr.is_null() {
                free_page(current_addr, current_order);
                break;
            }
            current_addr = current_addr.min(buddy_addr);
            current_order += 1;
        }
    }
}

fn add_region(base: u64, size: u64) -> () {
    let aligned_base: u64 = (base + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let aligned_size: u64 = size & !(PAGE_SIZE - 1);

    let mut base = aligned_base;
    let mut remaining = aligned_size;
    let kernel_start_addr = unsafe { &kernel_start as *const u8 as u64 };
    let kernel_end_addr = unsafe { &kernel_end as *const u8 as u64 };
    while remaining > 0 {
        let mut best_order: usize = 0;
        if base < 0x100000 {
            base += PAGE_SIZE << best_order;
            remaining -= PAGE_SIZE << best_order;
            continue;
        }
        for order in (0..MAX_ORDER).rev() {
            if PAGE_SIZE << order <= remaining && base % (PAGE_SIZE << order) == 0 {
                best_order = order;
                break;
            }
        }
        if base + (PAGE_SIZE << best_order) > kernel_start_addr && base < kernel_end_addr {
            base += PAGE_SIZE << best_order;
            remaining -= PAGE_SIZE << best_order;
            continue;
        }
        free_page(base, best_order);
        base += PAGE_SIZE << best_order;
        remaining -= PAGE_SIZE << best_order;
    }
}

pub fn init(multiboot2_info: *const u8) -> () {
    unsafe {
        let mut tag_ptr = multiboot2_info.add(8);
        let mut tag_type = *(tag_ptr.add(0) as *const u32);
        let mut tag_size: u32;
        while tag_type != 0 {
            tag_type = *(tag_ptr.add(0) as *const u32);
            tag_size = *(tag_ptr.add(4) as *const u32);
            if tag_type != 6 {
                tag_ptr = tag_ptr.add(((tag_size + 7) & !7) as usize);
                continue;
            }
            // we have the memory block thing tag
            let mut entry_ptr = tag_ptr.add(16);
            let mut bytes_read: u32 = 0;
            let entry_size = *(tag_ptr.add(8) as *const u32);

            while bytes_read < tag_size - 16 {
                let base_addr = entry_ptr.add(0) as *const u64;
                let length = entry_ptr.add(8) as *const u64;
                let entry_type = entry_ptr.add(16) as *const u32;

                if *entry_type == 1 {
                    add_region(*base_addr, *length)
                }
                entry_ptr = entry_ptr.add(entry_size as usize);
                bytes_read += entry_size;
            }

            tag_ptr = tag_ptr.add(((tag_size + 7) & !7) as usize);
        }
    }
}
