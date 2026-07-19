#pragma once
#define NEFIFB 0

struct pci_attach_args;

static inline int
efifb_is_primary(struct pci_attach_args *pa)
{
	(void)pa;
	return 0;
}

static inline int
efifb_is_console(struct pci_attach_args *pa)
{
	(void)pa;
	return 0;
}

static inline void
efifb_detach(void)
{
}

static inline void
efifb_reattach(void)
{
}
