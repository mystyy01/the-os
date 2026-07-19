#pragma once
#include <stdint.h>

#define PMAP_WIRED 0x10
#define PMAP_CANFAIL 0x20

struct pmap;
typedef struct pmap *pmap_t;

static inline struct pmap *
pmap_kernel(void)
{
	return (struct pmap *)0;
}

static inline void
pmap_kenter_pa(uint64_t va, uint64_t pa, int prot)
{
	(void)va;
	(void)pa;
	(void)prot;
}

static inline void
pmap_kremove(uint64_t va, uint64_t size)
{
	(void)va;
	(void)size;
}

static inline void
pmap_update(struct pmap *pm)
{
	(void)pm;
}

static inline int
pmap_enter(struct pmap *pm, uint64_t va, uint64_t pa, int prot, int flags)
{
	(void)pm;
	(void)va;
	(void)pa;
	(void)prot;
	(void)flags;
	return 0;
}

static inline int
pmap_extract(struct pmap *pm, uint64_t va, uint64_t *pap)
{
	(void)pm;
	(void)va;
	if (pap)
		*pap = 0;
	return 0;
}

static inline void
pmap_remove(struct pmap *pm, uint64_t start, uint64_t end)
{
	(void)pm;
	(void)start;
	(void)end;
}

#define PG_V 0x001
#define PG_RW 0x002
#define PG_U 0x004
#define PG_WT 0x008
#define PG_N 0x010
#define PG_PAT 0x080
#define PG_G 0x100

#define PMAP_NOCACHE 0x1
#define PMAP_WC 0x2

#define PROT_NONE 0x0
#define PROT_READ 0x1
#define PROT_WRITE 0x2
#define PROT_EXEC 0x4

struct vm_page;

static inline void
pmap_page_protect(struct vm_page *pg, int prot)
{
	(void)pg;
	(void)prot;
}
