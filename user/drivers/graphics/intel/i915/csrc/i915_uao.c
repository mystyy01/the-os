
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <sys/rwlock.h>
#include <uvm/uvm_extern.h>

extern void *compat_arena_alloc(size_t size);
extern void *vm_page_to_virt(void *pg);
extern void os_print(const char *s);

static void
dbg_u64(const char *label, uint64_t v)
{
	char buf[80];
	int i = 0;
	os_print("\x1b[33m");
	while (label[i] && i < 40) {
		buf[i] = label[i];
		i++;
	}
	buf[i++] = '0';
	buf[i++] = 'x';
	for (int s = 60; s >= 0; s -= 4) {
		int nib = (v >> s) & 0xf;
		buf[i++] = nib < 10 ? '0' + nib : 'a' + (nib - 10);
	}
	buf[i] = 0;
	os_print(buf);
	os_print("\x1b[0m");
}

struct uao_impl {
	struct uvm_object uobj;
	struct rwlock lock;
	uint64_t size;
	struct vm_page *pages;
	int npages;
};

static int
uao_pgo_flush(struct uvm_object *uo, voff_t start, voff_t stop, int flags)
{
	(void)uo;
	(void)start;
	(void)stop;
	(void)flags;
	return 1;
}

static void
uao_pgo_reference(struct uvm_object *uo)
{
	if (uo)
		uo->uo_refs++;
}

static void
uao_pgo_detach(struct uvm_object *uo)
{
	if (uo && uo->uo_refs > 0)
		uo->uo_refs--;
}

static const struct uvm_pagerops uao_pgops = {
	.pgo_reference = uao_pgo_reference,
	.pgo_detach = uao_pgo_detach,
	.pgo_fault = NULL,
	.pgo_flush = uao_pgo_flush,
};

struct uvm_object *
uao_create(uint64_t size, int flags)
{
	(void)flags;
	dbg_u64("uao_create size=", size);
	struct uao_impl *u = compat_arena_alloc(sizeof(*u));
	if (u == NULL)
		return NULL;
	memset(u, 0, sizeof(*u));
	u->size = size;
	u->uobj.pgops = &uao_pgops;
	u->uobj.vmobjlock = &u->lock;
	u->uobj.uo_refs = 1;
	return &u->uobj;
}

void
uao_reference(void *uao)
{
	struct uvm_object *uo = uao;
	if (uo)
		uo->uo_refs++;
}

void
uao_detach(struct uvm_object *uao)
{
	if (uao && uao->uo_refs > 0)
		uao->uo_refs--;
}

static int
uao_ensure_pages(struct uao_impl *u)
{
	if (u->pages != NULL)
		return 0;

	int npages = (int)((u->size + PAGE_SIZE - 1) / PAGE_SIZE);
	if (npages <= 0)
		return -1;

	struct vm_page *base = uvm_pagealloc((size_t)npages);
	if (base == NULL)
		return -1;


	for (int i = 0; i < npages; i++) {
		void *va = vm_page_to_virt(&base[i]);
		if (va != NULL)
			memset(va, 0, PAGE_SIZE);
	}

	u->pages = base;
	u->npages = npages;
	return 0;
}

int
uvm_obj_wire(struct uvm_object *uo, uint64_t start, uint64_t end,
    struct pglist *list)
{
	struct uao_impl *u = (struct uao_impl *)uo;
	if (u == NULL)
		return -1;

	dbg_u64("uvm_obj_wire size=", u->size);
	if (uao_ensure_pages(u) != 0) {
		os_print("\x1b[31muvm_obj_wire: uao_ensure_pages FAILED\x1b[0m");
		return -1;
	}
	dbg_u64("uvm_obj_wire ok npages=", (uint64_t)u->npages);

	int first = (int)(start / PAGE_SIZE);
	int last = (int)((end + PAGE_SIZE - 1) / PAGE_SIZE);
	if (last > u->npages)
		last = u->npages;
	if (first < 0)
		first = 0;

	for (int i = first; i < last; i++)
		TAILQ_INSERT_TAIL(list, &u->pages[i], pageq);

	return 0;
}

void
uvm_obj_unwire(struct uvm_object *uo, uint64_t start, uint64_t end)
{

	(void)uo;
	(void)start;
	(void)end;
}
