#include <stdint.h>
#include <string.h>
#include <dev/pci/pcivar.h>
#include <dev/pci/pcireg.h>
#include <sys/device.h>
#include "i915_drv.h"

int inteldrm_match(struct device *, void *, void *);
void inteldrm_attach(struct device *, struct device *, void *);

extern unsigned long __drm_debug;

static struct drm_i915_private g_i915;

int
i915_shim_probe(uint32_t bus, uint32_t device, uint32_t function,
    uint32_t vendor_id, uint32_t product_id,
    uint32_t pci_class, uint32_t pci_subclass, uint32_t pci_progif)
{
	struct pci_attach_args pa;

	__drm_debug = 0x7UL;

	memset(&pa, 0, sizeof(pa));
	pa.pa_pc = 0;
	pa.pa_tag = pci_make_tag(pa.pa_pc, (int)bus, (int)device, (int)function);
	pa.pa_bus = (int)bus;
	pa.pa_device = (int)device;
	pa.pa_function = (int)function;
	pa.pa_id = (vendor_id & 0xFFFFu) | (product_id << 16);
	pa.pa_class = (pci_class << PCI_CLASS_SHIFT) |
	    (pci_subclass << PCI_SUBCLASS_SHIFT) |
	    (pci_progif << PCI_INTERFACE_SHIFT);
	pa.pa_memt = 0;
	pa.pa_iot = 0;
	pa.pa_dmat = 0;
	pa.pa_memex = 0;
	pa.pa_flags = 0;

	if (inteldrm_match(0, 0, &pa) == 0)
		return 1;

	printf("i915: stage shim_before_attach\n");
	inteldrm_attach(0, (struct device *)&g_i915, &pa);
	printf("i915: stage shim_after_attach\n");
	return 0;
}

int
i915_shim_fb_info(uint32_t *width, uint32_t *height, uint32_t *depth,
    uint32_t *stride)
{
	struct rasops_info *ri = &g_i915.ro;

	if (ri->ri_bits == NULL)
		return -1;

	*width = ri->ri_width;
	*height = ri->ri_height;
	*depth = ri->ri_depth;
	*stride = ri->ri_stride;
	return 0;
}

int
i915_shim_draw_square(uint32_t x, uint32_t y, uint32_t w, uint32_t h,
    uint32_t rgb)
{
	struct rasops_info *ri = &g_i915.ro;
	uint8_t *base;
	uint32_t row, col;

	if (ri->ri_bits == NULL)
		return -1;
	if (ri->ri_depth != 32)
		return -2;
	if (x + w > ri->ri_width || y + h > ri->ri_height)
		return -3;

	base = (uint8_t *)ri->ri_bits;
	for (row = 0; row < h; row++) {
		uint32_t *line = (uint32_t *)(base + (y + row) * ri->ri_stride);
		for (col = 0; col < w; col++)
			line[x + col] = rgb;
	}
	return 0;
}
