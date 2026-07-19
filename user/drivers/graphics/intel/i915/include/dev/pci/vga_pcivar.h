#pragma once
#include <machine/bus.h>

struct vga_pci_bar {
	bus_space_tag_t bst;
	bus_space_handle_t bsh;
	bus_addr_t base;
	bus_size_t size;
};
