#pragma once
#include <stddef.h>
#include <stdint.h>
#include <sys/queue.h>
#include <sys/types.h>

#define PAGE_SHIFT 12
#define PAGE_SIZE (1UL << PAGE_SHIFT)
#define PAGE_MASK (PAGE_SIZE - 1)
#define round_page(x) (((x) + PAGE_MASK) & ~PAGE_MASK)
#define trunc_page(x) ((x) & ~PAGE_MASK)

struct vm_map;

struct kmem_va_mode {
	struct vm_map **kv_map;
	int kv_wait;
};
struct kmem_pa_mode {
	int dummy;
};
struct kmem_dyn_mode {
	int dummy;
};

extern struct kmem_va_mode kv_page;
extern struct kmem_va_mode kv_any;
extern struct kmem_pa_mode kp_dirty;
extern struct kmem_pa_mode kp_zero;
extern struct kmem_pa_mode kp_none;
extern struct kmem_dyn_mode kd_nowait;
extern struct kmem_dyn_mode kd_waitok;
extern struct vm_map *phys_map;

void *km_alloc(size_t size, const struct kmem_va_mode *kv,
    const struct kmem_pa_mode *kp, const struct kmem_dyn_mode *kd);
void km_free(void *addr, size_t size, const struct kmem_va_mode *kv,
    const struct kmem_pa_mode *kp);

struct vm_page {
	uint64_t phys_addr;
	void *virt_addr;
	unsigned int pg_flags;
	TAILQ_ENTRY(vm_page) pageq;
};

TAILQ_HEAD(pglist, vm_page);

struct uvm_constraint_range {
	uint64_t ucr_low;
	uint64_t ucr_high;
};

extern struct uvm_constraint_range no_constraint;
extern struct uvm_constraint_range dma_constraint;

#define UVM_PLA_NOWAIT 0x01
#define UVM_PLA_WAITOK 0x02
#define UVM_PLA_FAILOK 0x04
#define UVM_PLA_ZERO 0x08

static inline int
uvm_pglistalloc(uint64_t size, uint64_t low, uint64_t high, uint64_t alignment,
    uint64_t boundary, struct pglist *rlist, int nsegs, int waitok)
{
	(void)size;
	(void)low;
	(void)high;
	(void)alignment;
	(void)boundary;
	(void)rlist;
	(void)nsegs;
	(void)waitok;
	return -1;
}

static inline void
uvm_pglistfree(struct pglist *list)
{
	(void)list;
}

#define PG_PMAP_WC 0x0001
#define PG_CLEAN 0x0002

#define atop(x) ((x) >> PAGE_SHIFT)
#define ptoa(x) ((x) << PAGE_SHIFT)

extern struct vm_map *kernel_map;

static inline uint64_t
vm_map_min(struct vm_map *map)
{
	(void)map;
	return 0;
}

static inline uint64_t
vm_map_max(struct vm_map *map)
{
	(void)map;
	return ~0ULL;
}

#define PHYSLOAD_DEVICE 0x01

static inline void
uvm_page_physload(uint64_t start, uint64_t end, uint64_t avail_start,
    uint64_t avail_end, int flags)
{
	(void)start;
	(void)end;
	(void)avail_start;
	(void)avail_end;
	(void)flags;
}

struct uvmexp_s {
	int free;
	int npages;
	int swpages;
	int swpginuse;
};
extern struct uvmexp_s uvmexp;

struct vm_page *uvm_atopg(uint64_t kva);
struct vm_page *uvm_pagealloc(size_t npages);
uint64_t vm_page_to_phys(struct vm_page *pg);
struct vm_page *phys_to_vm_page(uint64_t pa);

#define VM_PAGE_TO_PHYS(pg) vm_page_to_phys(pg)
#define PHYS_TO_VM_PAGE(pa) phys_to_vm_page(pa)
