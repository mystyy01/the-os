#include <stdint.h>
#include <dev/pci/pcivar.h>
#include <dev/pci/pcireg.h>
#include <sys/device.h>
#include "i915_drv.h"

int inteldrm_match(struct device *, void *, void *);
void inteldrm_attach(struct device *, struct device *, void *);

static struct drm_i915_private g_i915;

int
i915_shim_probe(uint32_t bus, uint32_t device, uint32_t function,
    uint32_t vendor_id, uint32_t product_id,
    uint32_t pci_class, uint32_t pci_subclass, uint32_t pci_progif)
{
	struct pci_attach_args pa;

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

	inteldrm_attach(0, (struct device *)&g_i915, &pa);
	return 0;
}
