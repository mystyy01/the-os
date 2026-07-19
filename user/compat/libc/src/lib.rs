#![no_std]

use core::ffi::{c_int, c_void};

const ARENA_SIZE: usize = 16 * 1024 * 1024;
static mut ARENA: [u8; ARENA_SIZE] = [0; ARENA_SIZE];
static mut OFFSET: usize = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    let off = (OFFSET + 15) & !15;
    if off + size > ARENA_SIZE {
        return core::ptr::null_mut();
    }
    OFFSET = off + size;
    (&raw mut ARENA as *mut u8).add(off) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    let total = nmemb * size;
    let p = malloc(total);
    if !p.is_null() {
        core::ptr::write_bytes(p as *mut u8, 0, total);
    }
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(_ptr: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compat_arena_alloc(size: usize) -> *mut c_void {
    malloc(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compat_arena_free(_ptr: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn abort() -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut i = 0;
    while *s.add(i) != 0 {
        i += 1;
    }
    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strnlen(s: *const u8, maxlen: usize) -> usize {
    let mut i = 0;
    while i < maxlen && *s.add(i) != 0 {
        i += 1;
    }
    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcmp(a: *const u8, b: *const u8) -> c_int {
    let mut i = 0;
    loop {
        let (ca, cb) = (*a.add(i), *b.add(i));
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncmp(a: *const u8, b: *const u8, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let (ca, cb) = (*a.add(i), *b.add(i));
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcpy(dst: *mut u8, src: *const u8) -> *mut u8 {
    let mut i = 0;
    loop {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlcpy(dst: *mut u8, src: *const u8, siz: usize) -> usize {
    let srclen = strlen(src);
    if siz != 0 {
        let n = if srclen + 1 < siz { srclen } else { siz - 1 };
        core::ptr::copy_nonoverlapping(src, dst, n);
        *dst.add(n) = 0;
    }
    srclen
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlcat(dst: *mut u8, src: *const u8, siz: usize) -> usize {
    let dstlen = strlen(dst);
    let srclen = strlen(src);
    if dstlen >= siz {
        return dstlen + srclen;
    }
    let avail = siz - dstlen - 1;
    let n = if srclen < avail { srclen } else { avail };
    core::ptr::copy_nonoverlapping(src, dst.add(dstlen), n);
    *dst.add(dstlen + n) = 0;
    dstlen + srclen
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 {
            i += 1;
            break;
        }
        i += 1;
    }
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strchr(s: *const u8, c: c_int) -> *mut u8 {
    let mut i = 0;
    loop {
        let cur = *s.add(i);
        if cur as c_int == c {
            return s.add(i) as *mut u8;
        }
        if cur == 0 {
            return core::ptr::null_mut();
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strrchr(s: *const u8, c: c_int) -> *mut u8 {
    let mut result: *mut u8 = core::ptr::null_mut();
    let mut i = 0;
    loop {
        let cur = *s.add(i);
        if cur as c_int == c {
            result = s.add(i) as *mut u8;
        }
        if cur == 0 {
            break;
        }
        i += 1;
    }
    result
}

type CmpFn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qsort(base: *mut c_void, nmemb: usize, size: usize, cmp: CmpFn) {
    if nmemb < 2 {
        return;
    }
    let b = base as *mut u8;
    let mut tmp = [0u8; 256];
    if size > tmp.len() {
        return;
    }
    for i in 1..nmemb {
        core::ptr::copy_nonoverlapping(b.add(i * size), tmp.as_mut_ptr(), size);
        let mut j = i;
        while j > 0
            && cmp(
                b.add((j - 1) * size) as *const c_void,
                tmp.as_ptr() as *const c_void,
            ) > 0
        {
            core::ptr::copy_nonoverlapping(b.add((j - 1) * size), b.add(j * size), size);
            j -= 1;
        }
        core::ptr::copy_nonoverlapping(tmp.as_ptr(), b.add(j * size), size);
    }
}
