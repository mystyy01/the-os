#pragma once
#include <machine/bus.h>

struct vga_config {
	int dummy;
};

static inline int
vga_is_console(bus_space_tag_t iot, int type)
{
	(void)iot;
	(void)type;
	return 0;
}
