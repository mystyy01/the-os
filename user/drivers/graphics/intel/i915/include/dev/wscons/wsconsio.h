#pragma once

#define WSDISPLAYIO_GTYPE 1
#define WSDISPLAYIO_GINFO 2
#define WSDISPLAYIO_GETPARAM 3
#define WSDISPLAYIO_SETPARAM 4
#define WSDISPLAYIO_SVIDEO 5
#define WSDISPLAYIO_GVIDEO 6

#define WSDISPLAYIO_PARAM_BRIGHTNESS 1

#define WSDISPLAY_TYPE_INTELDRM 42
#define WSDISPLAY_BURN_VBLANK 0x01

struct wsdisplay_fbinfo {
	unsigned int width;
	unsigned int height;
	unsigned int depth;
	unsigned int cmsize;
	unsigned int stride;
	unsigned int offset;
};

struct wsdisplay_param {
	int param;
	int min;
	int max;
	int curval;
};

struct wsdisplay_font {
	int fontwidth;
	int fontheight;
};

struct wsdisplay_charcell {
	int dummy;
};

extern int (*ws_get_param)(struct wsdisplay_param *);
extern int (*ws_set_param)(struct wsdisplay_param *);
