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

static inline int
extent_alloc_subregion(struct extent *ex, unsigned long substart,
    unsigned long subend, unsigned long size, unsigned long alignment,
    unsigned long skew, unsigned long boundary, int flags,
    unsigned long *result)
{
	(void)ex;
	(void)substart;
	(void)subend;
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
void i915_irq_register(int (*func)(void *), void *arg);
void os_print(const char *s);

static inline int
pci_find_device(struct pci_attach_args *pa, int (*match)(struct pci_attach_args *))
{
	struct pci_attach_args probe = {0};

	probe.pa_id = 0x8086u | (0x7A80u << 16);
	probe.pa_class = (0x06u << 24) | (0x01u << 16);

	if (match(&probe)) {
		*pa = probe;
		return 1;
	}
	return 0;
}

extern uint64_t os_map_mmio(uint64_t phys, uint64_t pages);

static inline int
pci_intr_map_msi(struct pci_attach_args *pa, pci_intr_handle_t *ihp)
{
	int cap;
	uint32_t ctl;
	int data_reg;

	if (!pci_get_capability(pa->pa_pc, pa->pa_tag, 0x05, &cap, NULL))
		return 1;
	os_print("i915: MSI capability found\n");
	ctl = pci_conf_read(pa->pa_pc, pa->pa_tag, cap);
	pci_conf_write(pa->pa_pc, pa->pa_tag, cap + 4, 0xfee00000);
	if (ctl & (1U << 23)) {
		pci_conf_write(pa->pa_pc, pa->pa_tag, cap + 8, 0);
		data_reg = cap + 12;
	} else {
		data_reg = cap + 8;
	}
	uint32_t data = pci_conf_read(pa->pa_pc, pa->pa_tag, data_reg & ~3);
	data &= ~(0xffffU << ((data_reg & 2) * 8));
	data |= 65U << ((data_reg & 2) * 8);
	pci_conf_write(pa->pa_pc, pa->pa_tag, data_reg & ~3, data);
	pci_conf_write(pa->pa_pc, pa->pa_tag, cap, ctl | (1U << 16));
	os_print("i915: MSI vector 65 enabled\n");
	pa->pa_flags |= PCI_FLAGS_MSI_ENABLED;
	if (ihp)
		*ihp = 65;
	return 0;
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
	return ih == 65 ? "msi65" : "irq0";
}

static inline void *
pci_intr_establish(pci_chipset_tag_t pc, pci_intr_handle_t ih, int level,
    int (*func)(void *), void *arg, const char *name)
{
	(void)pc;
	(void)ih;
	(void)level;
	i915_irq_register(func, arg);
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
	pcireg_t lo, hi = 0;
	bus_addr_t phys;
	uint64_t size;
	int is64 = ((type & 0x6) == 0x4);

	lo = pci_conf_read(pc, tag, reg);
	if (is64) {
		hi = pci_conf_read(pc, tag, reg + 4);
		phys = ((uint64_t)hi << 32) | (lo & ~0xFUL);
	} else {
		phys = lo & ~0xFUL;
	}

	pci_conf_write(pc, tag, reg, 0xFFFFFFFFu);
	uint32_t masklo = pci_conf_read(pc, tag, reg);
	pci_conf_write(pc, tag, reg, lo);

	if (is64) {
		pci_conf_write(pc, tag, reg + 4, 0xFFFFFFFFu);
		uint32_t maskhi = pci_conf_read(pc, tag, reg + 4);
		pci_conf_write(pc, tag, reg + 4, hi);
		uint64_t mask = ((uint64_t)maskhi << 32) | (uint32_t)(masklo & ~0xFu);
		size = ~mask + 1;
	} else {
		uint32_t mask = masklo & ~0xFu;
		size = (uint64_t)((uint32_t)(~mask + 1));
	}

	if (basep)
		*basep = phys;
	if (sizep)
		*sizep = size;
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
