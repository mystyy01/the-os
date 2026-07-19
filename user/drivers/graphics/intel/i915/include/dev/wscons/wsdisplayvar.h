#pragma once
#include <stdint.h>
#include <sys/types.h>
#include <dev/wscons/wsconsio.h>

#define WSSCREEN_UNDERLINE 0x01
#define WSSCREEN_HILIT 0x02
#define WSSCREEN_REVERSE 0x04
#define WSSCREEN_WSCOLORS 0x08

struct wsdisplay_emulops {
	void (*pack_attr)(void *cookie, int fg, int bg, int flags, uint32_t *attrp);
};

struct wsscreen_descr {
	const char *name;
	int ncols;
	int nrows;
	const struct wsdisplay_emulops *textops;
	int fontwidth;
	int fontheight;
	int capabilities;
};

struct wsscreen_list {
	int nscreens;
	const struct wsscreen_descr **screens;
};

struct wsdisplay_accessops {
	int (*ioctl)(void *, unsigned long, char *, int, struct proc *);
	long (*mmap)(void *, off_t, int);
	int (*alloc_screen)(void *, const struct wsscreen_descr *,
	    void **, int *, int *, uint32_t *);
	void (*free_screen)(void *, void *);
	int (*show_screen)(void *, void *, int,
	    void (*)(void *, int, int), void *);
	void (*enter_ddb)(void *, void *);
	int (*getchar)(void *, int, int, struct wsdisplay_charcell *);
	int (*load_font)(void *, void *, struct wsdisplay_font *);
	int (*list_font)(void *, struct wsdisplay_font *);
	void (*scrollback)(void *, void *, int);
	void (*burn_screen)(void *, unsigned int, unsigned int);
};

struct wsemuldisplaydev_attach_args {
	int console;
	int primary;
	struct wsscreen_list *scrdata;
	struct wsdisplay_accessops *accessops;
	void *accesscookie;
	int defaultscreens;
};

int wsemuldisplaydevprint(void *aux, const char *pnp);
int wsemuldisplaydevsubmatch(struct device *parent, void *match, void *aux);
int wsdisplay_cnattach(const struct wsscreen_descr *, void *, int, int, uint32_t);
