#pragma once
#include <stdint.h>
#include <machine/bus.h>

typedef int pci_chipset_tag_t;
typedef uint32_t pcitag_t;
typedef uint32_t pcireg_t;
typedef int pci_intr_handle_t;

struct extent;

struct pci_softc {
	pcitag_t *sc_bridgetag;
};

static inline int
extent_alloc(struct extent *ex, unsigned long size, unsigned long alignment,
    unsigned long skew, unsigned long boundary, int flags,
    unsigned long *result)
{
	(void)ex;
	(void)size;
	(void)alignment;
	(void)skew;
	(void)boundary;
	(void)flags;
	(void)result;
	return -1;
}

static inline void
extent_free(struct extent *ex, unsigned long start, unsigned long size,
    int flags)
{
	(void)ex;
	(void)start;
	(void)size;
	(void)flags;
}

struct pci_attach_args {
	pci_chipset_tag_t pa_pc;
	pcitag_t pa_tag;
	int pa_bus;
	int pa_device;
	int pa_function;
	uint32_t pa_id;
	uint32_t pa_class;
	int pa_memt;
	int pa_iot;
	int pa_dmat;
	struct extent *pa_memex;
	int pa_flags;
	int pa_domain;
	pcitag_t *pa_bridgetag;
};

#define PCI_FLAGS_MSI_ENABLED 0x01

int pci_enumerate_bus(struct pci_softc *sc,
    int (*match)(struct pci_attach_args *), struct pci_attach_args *pa);

static inline pcitag_t
pci_make_tag(pci_chipset_tag_t pc, int bus, int dev, int func)
{
	(void)pc;
	return ((uint32_t)bus << 16) | ((uint32_t)dev << 11) | ((uint32_t)func << 8);
}

static inline void
pci_decompose_tag(pci_chipset_tag_t pc, pcitag_t tag, int *bus, int *dev, int *func)
{
	(void)pc;
	if (bus)
		*bus = (tag >> 16) & 0xFF;
	if (dev)
		*dev = (tag >> 11) & 0x1F;
	if (func)
		*func = (tag >> 8) & 0x7;
}

uint32_t pci_conf_read(pci_chipset_tag_t pc, pcitag_t tag, int reg);
void pci_conf_write(pci_chipset_tag_t pc, pcitag_t tag, int reg, uint32_t data);
int pci_get_capability(pci_chipset_tag_t pc, pcitag_t tag, int cap, int *offset, uint32_t *value);

static inline int
pci_find_device(struct pci_attach_args *pa, int (*match)(struct pci_attach_args *))
{
	(void)pa;
	(void)match;
	return 0;
}

extern uint64_t os_map_mmio(uint64_t phys, uint64_t pages);

static inline int
pci_intr_map_msi(struct pci_attach_args *pa, pci_intr_handle_t *ihp)
{
	(void)pa;
	(void)ihp;
	return 1;
}

static inline int
pci_intr_map(struct pci_attach_args *pa, pci_intr_handle_t *ihp)
{
	(void)pa;
	if (ihp)
		*ihp = 0;
	return 0;
}

static inline const char *
pci_intr_string(pci_chipset_tag_t pc, pci_intr_handle_t ih)
{
	(void)pc;
	(void)ih;
	return "irq0";
}

static inline void *
pci_intr_establish(pci_chipset_tag_t pc, pci_intr_handle_t ih, int level,
    int (*func)(void *), void *arg, const char *name)
{
	(void)pc;
	(void)ih;
	(void)level;
	(void)func;
	(void)arg;
	(void)name;
	return (void *)1;
}

static inline pcireg_t
pci_mapreg_type(pci_chipset_tag_t pc, pcitag_t tag, int reg)
{
	return pci_conf_read(pc, tag, reg) & 0xf;
}

static inline int
pci_mapreg_info(pci_chipset_tag_t pc, pcitag_t tag, int reg, pcireg_t type,
    bus_addr_t *basep, bus_size_t *sizep, int *flagsp)
{
	pcireg_t lo, hi;
	bus_addr_t phys;

	lo = pci_conf_read(pc, tag, reg);
	if ((type & 0x6) == 0x4) {
		hi = pci_conf_read(pc, tag, reg + 4);
		phys = ((uint64_t)hi << 32) | (lo & ~0xFUL);
	} else {
		phys = lo & ~0xFUL;
	}

	if (basep)
		*basep = phys;
	if (sizep)
		*sizep = 0;
	if (flagsp)
		*flagsp = 0;
	return 0;
}

static inline int
pci_mapreg_map(struct pci_attach_args *pa, int reg, pcireg_t type,
    int flags, bus_space_tag_t *tagp, bus_space_handle_t *handlep,
    bus_addr_t *basep, bus_size_t *sizep, bus_size_t maxsize)
{
	pcireg_t lo, hi;
	bus_addr_t phys;
	uint64_t vaddr;
	uint64_t pages;

	(void)flags;

	lo = pci_conf_read(pa->pa_pc, pa->pa_tag, reg);
	if ((type & 0x6) == 0x4) {
		hi = pci_conf_read(pa->pa_pc, pa->pa_tag, reg + 4);
		phys = ((uint64_t)hi << 32) | (lo & ~0xFUL);
	} else {
		phys = lo & ~0xFUL;
	}

	pages = (maxsize + 4095) / 4096;
	vaddr = os_map_mmio(phys, pages);
	if (vaddr == 0)
		return 1;

	if (tagp)
		*tagp = 0;
	if (handlep)
		*handlep = vaddr;
	if (basep)
		*basep = phys;
	if (sizep)
		*sizep = maxsize;
	return 0;
}
