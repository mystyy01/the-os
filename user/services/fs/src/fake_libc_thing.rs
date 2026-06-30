use core::ffi::{c_int, c_void};

const ARENA_SIZE: usize = 8 * 1024 * 1024;
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
        core::ptr::write_bytes(p as *mut u8, 0, total); // arena is already zero, but be correct
    }
    return p;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(_ptr: *mut c_void) {
    // lol we gonna get hella memory leaks for now
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
    return 0;
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
    return dst;
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
