use core::sync::atomic::{AtomicBool, Ordering};

use crate::vmm::phys_to_virt;

static FONT: &[u8] = include_bytes!("font.psf");

#[derive(Copy, Clone)]
pub struct FbInfo {
    pub phys: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
}

static mut FB: FbInfo = FbInfo {
    phys: 0,
    pitch: 0,
    width: 0,
    height: 0,
    bpp: 0,
};
static PRESENT: AtomicBool = AtomicBool::new(false);

static mut CUR_COL: u32 = 0;
static mut CUR_ROW: u32 = 0;
static mut CRASH_COL: u32 = 0;
static mut CRASH_ROW: u32 = 0;

const FG: u32 = 0x00d0d0d0;
const BG: u32 = 0x00000000;

fn font_header(off: usize) -> u32 {
    u32::from_le_bytes([FONT[off], FONT[off + 1], FONT[off + 2], FONT[off + 3]])
}

fn glyph_h() -> u32 {
    font_header(24)
}

fn glyph_w() -> u32 {
    font_header(28)
}

fn glyph(c: u8) -> &'static [u8] {
    let headersize = font_header(8) as usize;
    let bytes_per_glyph = font_header(20) as usize;
    let start = headersize + c as usize * bytes_per_glyph;
    &FONT[start..start + bytes_per_glyph]
}

pub fn init(multiboot2_info: *const u8) {
    unsafe {
        let mut tag_ptr = multiboot2_info.add(8);
        loop {
            let tag_type = *(tag_ptr.add(0) as *const u32);
            let tag_size = *(tag_ptr.add(4) as *const u32);
            if tag_type == 0 {
                return;
            }
            if tag_type == 8 {
                let fb_type = *tag_ptr.add(29);
                if fb_type != 1 {
                    return;
                }
                FB = FbInfo {
                    phys: *(tag_ptr.add(8) as *const u64),
                    pitch: *(tag_ptr.add(16) as *const u32),
                    width: *(tag_ptr.add(20) as *const u32),
                    height: *(tag_ptr.add(24) as *const u32),
                    bpp: *tag_ptr.add(28),
                };
                PRESENT.store(true, Ordering::Release);
                clear();
                return;
            }
            tag_ptr = tag_ptr.add(((tag_size + 7) & !7) as usize);
        }
    }
}

pub fn present() -> bool {
    PRESENT.load(Ordering::Acquire)
}

pub fn info() -> FbInfo {
    unsafe { FB }
}

fn base() -> *mut u8 {
    unsafe { phys_to_virt(FB.phys) as *mut u8 }
}

pub fn put_pixel(x: u32, y: u32, color: u32) {
    unsafe {
        if x >= FB.width || y >= FB.height {
            return;
        }
        let off = y as usize * FB.pitch as usize + x as usize * (FB.bpp as usize / 8);
        let p = base().add(off);
        match FB.bpp {
            32 => *(p as *mut u32) = color,
            24 => {
                *p = color as u8;
                *p.add(1) = (color >> 8) as u8;
                *p.add(2) = (color >> 16) as u8;
            }
            _ => {}
        }
    }
}

pub fn fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    unsafe {
        if FB.bpp == 32 {
            let x_end = core::cmp::min(x + w, FB.width);
            let y_end = core::cmp::min(y + h, FB.height);
            for row in y..y_end {
                let off = row as usize * FB.pitch as usize + x as usize * 4;
                let mut p = base().add(off) as *mut u32;
                for _ in x..x_end {
                    *p = color;
                    p = p.add(1);
                }
            }
        } else {
            for row in y..y + h {
                for col in x..x + w {
                    put_pixel(col, row, color);
                }
            }
        }
    }
}

pub fn clear() {
    unsafe {
        fill_rect(0, 0, FB.width, FB.height, BG);
        CUR_COL = 0;
        CUR_ROW = 0;
    }
}

static mut CUR_FG: u32 = FG;
static mut ESC_STATE: u8 = 0;
static mut ESC_NUM: u32 = 0;

fn apply_sgr(n: u32) {
    unsafe {
        CUR_FG = match n {
            31 => 0x00FF5050,
            32 => 0x0050FF50,
            33 => 0x00FFFF50,
            36 => 0x0050FFFF,
            _ => FG,
        };
    }
}

fn draw_glyph(c: u8, px: u32, py: u32) {
    let g = glyph(c);
    let w = glyph_w();
    let h = glyph_h();
    let bytes_per_row = (w as usize + 7) / 8;
    for row in 0..h {
        for col in 0..w {
            let byte = g[row as usize * bytes_per_row + col as usize / 8];
            let bit = byte >> (7 - (col % 8)) & 1;
            let color = if bit != 0 { unsafe { CUR_FG } } else { BG };
            put_pixel(px + col, py + row, color);
        }
    }
}

fn text_cols() -> u32 {
    unsafe { FB.width / glyph_w() }
}

fn text_rows() -> u32 {
    unsafe { FB.height / glyph_h() }
}

fn scroll() {
    unsafe {
        let line_bytes = glyph_h() as usize * FB.pitch as usize;
        let total = FB.height as usize * FB.pitch as usize;
        core::ptr::copy(base().add(line_bytes), base(), total - line_bytes);
        fill_rect(
            0,
            (text_rows() - 1) * glyph_h(),
            FB.width,
            FB.height - (text_rows() - 1) * glyph_h(),
            BG,
        );
    }
}

pub fn con_putc(byte: u8) {
    unsafe {
        match ESC_STATE {
            1 => {
                if byte == b'[' {
                    ESC_STATE = 2;
                    ESC_NUM = 0;
                } else {
                    ESC_STATE = 0;
                }
                return;
            }
            2 => {
                match byte {
                    b'0'..=b'9' => ESC_NUM = ESC_NUM * 10 + (byte - b'0') as u32,
                    b';' => {
                        apply_sgr(ESC_NUM);
                        ESC_NUM = 0;
                    }
                    b'm' => {
                        apply_sgr(ESC_NUM);
                        ESC_STATE = 0;
                    }
                    _ => ESC_STATE = 0,
                }
                return;
            }
            _ => {}
        }
        if byte == 0x1b {
            ESC_STATE = 1;
            return;
        }
        match byte {
            b'\n' => {
                CUR_COL = 0;
                CUR_ROW += 1;
            }
            b'\r' => {
                CUR_COL = 0;
            }
            _ => {
                draw_glyph(byte, CUR_COL * glyph_w(), CUR_ROW * glyph_h());
                CUR_COL += 1;
                if CUR_COL >= text_cols() {
                    CUR_COL = 0;
                    CUR_ROW += 1;
                }
            }
        }
        if CUR_ROW >= text_rows() {
            scroll();
            CUR_ROW = text_rows() - 1;
        }
    }
}

pub fn crash_begin() {
    unsafe {
        fill_rect(0, 0, FB.width, FB.height, BG);
        CRASH_COL = 0;
        CRASH_ROW = 0;
    }
}

pub fn crash_putc(byte: u8) {
    unsafe {
        match byte {
            b'\n' => {
                CRASH_COL = 0;
                CRASH_ROW += 1;
            }
            b'\r' => {
                CRASH_COL = 0;
            }
            _ => {
                draw_glyph(byte, CRASH_COL * glyph_w(), CRASH_ROW * glyph_h());
                CRASH_COL += 1;
                if CRASH_COL >= text_cols() {
                    CRASH_COL = 0;
                    CRASH_ROW += 1;
                }
            }
        }
        if CRASH_ROW >= text_rows() {
            scroll();
            CRASH_ROW = text_rows() - 1;
        }
    }
}
