#pragma once
#include <stdint.h>
#include <dev/wscons/wsdisplayvar.h>

#define RI_CENTER 0x0001
#define RI_WRONLY 0x0002
#define RI_VCONS 0x0004
#define RI_CLEAR 0x0008
#define RI_ROTATE_CW 0x0010
#define RI_ROTATE_CCW 0x0020

struct rasops_info {
	int ri_width;
	int ri_height;
	int ri_depth;
	int ri_stride;
	void *ri_active;
	unsigned char *ri_bits;
	int ri_flg;
	void *ri_hw;
	int ri_caps;
	int ri_rows;
	int ri_cols;
	int ri_rnum;
	int ri_rpos;
	int ri_gnum;
	int ri_gpos;
	int ri_bnum;
	int ri_bpos;
	struct wsdisplay_emulops ri_ops;
	struct wsdisplay_font *ri_font;
};

static inline int
rasops_alloc_screen(struct rasops_info *ri, void **cookiep, int *curxp,
    int *curyp, uint32_t *attrp)
{
	(void)ri;
	(void)cookiep;
	(void)curxp;
	(void)curyp;
	(void)attrp;
	return 0;
}

static inline void
rasops_free_screen(struct rasops_info *ri, void *cookie)
{
	(void)ri;
	(void)cookie;
}

static inline void
rasops_show_screen(struct rasops_info *ri, void *cookie, int waitok,
    void (*cb)(void *, int, int), void *cbarg)
{
	(void)ri;
	(void)cookie;
	(void)waitok;
	(void)cb;
	(void)cbarg;
}

static inline int
rasops_getchar(struct rasops_info *ri, int row, int col,
    struct wsdisplay_charcell *cell)
{
	(void)ri;
	(void)row;
	(void)col;
	(void)cell;
	return 0;
}

static inline int
rasops_load_font(struct rasops_info *ri, void *cookie,
    struct wsdisplay_font *font)
{
	(void)ri;
	(void)cookie;
	(void)font;
	return 0;
}

static inline int
rasops_list_font(struct rasops_info *ri, struct wsdisplay_font *font)
{
	(void)ri;
	(void)font;
	return 0;
}

static inline void
rasops_scrollback(struct rasops_info *ri, void *cookie, int lines)
{
	(void)ri;
	(void)cookie;
	(void)lines;
}

static inline int
rasops_init(struct rasops_info *ri, int cols, int rows)
{
	(void)ri;
	(void)cols;
	(void)rows;
	return 0;
}
