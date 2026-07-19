#pragma once
#include <sys/cdefs.h>
#include <sys/queue.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

struct cfdata;

struct device {
	int dv_class;
	int dv_unit;
	char dv_xname[16];
	struct device *dv_parent;
	int dv_flags;
	struct cfdata *dv_cfdata;
	void *dv_private;
};

typedef unsigned char u_char;
typedef unsigned short u_short;
typedef unsigned int u_int;
typedef unsigned long u_long;
typedef long register_t;
typedef long paddr_t;
typedef long vaddr_t;
typedef long psize_t;
typedef long vsize_t;
typedef long voff_t;
typedef unsigned int vm_prot_t;
typedef int vm_fault_t;
struct vm_page;
typedef struct vm_page *vm_page_t;
struct uvm_pagerops;
struct pmap;
struct rwlock;
struct pglist;

struct vm_map {
	struct pmap *pmap;
};

struct vm_aref {
	void *ar_amap;
};

struct vm_map_entry {
	unsigned long start;
	unsigned long end;
	uint64_t offset;
	unsigned int protection;
	struct {
		struct uvm_object *uvm_obj;
	} object;
	struct vm_aref aref;
};

#define UVM_ET_ISCOPYONWRITE(entry) (0)

typedef int boolean_t;
#define TRUE 1
#define FALSE 0

struct uvm_faultinfo {
	struct vm_map_entry *entry;
	struct vm_map *orig_map;
};

struct uvm_pagerops {
	void (*pgo_reference)(struct uvm_object *);
	void (*pgo_detach)(struct uvm_object *);
	int (*pgo_fault)(struct uvm_faultinfo *, vaddr_t, vm_page_t *, int,
	    int, vm_fault_t, vm_prot_t, int);
	boolean_t (*pgo_flush)(struct uvm_object *, voff_t, voff_t, int);
};

static inline void
uvmfault_unlockall(struct uvm_faultinfo *ufi, void *amap, void *uobj)
{
	(void)ufi;
	(void)amap;
	(void)uobj;
}

static inline int
uvm_map(struct vm_map *map, unsigned long *addrp, unsigned long size,
    void *uobj, uint64_t uoffset, unsigned long align, unsigned int flags)
{
	(void)map;
	(void)addrp;
	(void)size;
	(void)uobj;
	(void)uoffset;
	(void)align;
	(void)flags;
	return -1;
}

static inline void
uao_reference(void *uao)
{
	(void)uao;
}

#define UVM_MAPFLAG(prot, maxprot, inherit, advice, flags) (0)
#define MAP_INHERIT_SHARE 0
#define MADV_RANDOM 0
#define UVM_FLAG_WC 0x01

struct uvm_object {
	int dummy_lock;
	const struct uvm_pagerops *pgops;
	unsigned int uo_refs;
	int uo_npages;
	struct rwlock *vmobjlock;
};

#define PGO_ALLPAGES 0x001
#define PGO_FREE 0x008

static inline int
uvm_obj_wire(struct uvm_object *uo, uint64_t start, uint64_t end,
    struct pglist *list)
{
	(void)uo;
	(void)start;
	(void)end;
	(void)list;
	return -1;
}

static inline void
uvm_obj_unwire(struct uvm_object *uo, uint64_t start, uint64_t end)
{
	(void)uo;
	(void)start;
	(void)end;
}

static inline void
uao_detach(struct uvm_object *uao)
{
	(void)uao;
}

static inline void
uvm_obj_init(struct uvm_object *uo, const struct uvm_pagerops *ops, int refs)
{
	(void)ops;
	uo->uo_refs = refs;
}

static inline void
uvm_obj_destroy(struct uvm_object *uo)
{
	(void)uo;
}

static inline struct uvm_object *
uao_create(uint64_t size, int flags)
{
	(void)size;
	(void)flags;
	return 0;
}
typedef int64_t daddr_t;
typedef uint64_t blkcnt_t;
typedef long off_t;
typedef long __ptrdiff_t;
typedef long ssize_t;
typedef long time_t;
typedef int pid_t;
typedef unsigned int dev_t;
typedef char *caddr_t;
typedef unsigned int uint;
typedef unsigned short ushort;
typedef unsigned long ulong;
typedef unsigned long __uintptr_t;

#include <sys/device.h>
