pub struct KernelHeap;

use crate::pmm::MAX_ORDER;
use crate::pmm::PAGE_SIZE;
use crate::pmm::free_pages;

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use crate::pmm::alloc_pages;

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut curr_order: usize = 0;
        let size = layout.size();
        loop {
            if curr_order >= MAX_ORDER {
                return null_mut();
            }
            if PAGE_SIZE << curr_order >= size as u64 {
                return alloc_pages(curr_order);
            }
            curr_order += 1;
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut curr_order: usize = 0;
        let size = layout.size();
        loop {
            if curr_order >= MAX_ORDER {
                return;
            }
            if PAGE_SIZE << curr_order >= size as u64 {
                free_pages(curr_order, ptr as u64);
                return;
            }
            curr_order += 1;
        }
    }
}

#[global_allocator]
static HEAP: KernelHeap = KernelHeap;
