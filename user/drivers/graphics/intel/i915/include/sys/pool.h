#pragma once
#include <stddef.h>
#include <sys/types.h>

#define PR_WAITOK 0x0001
#define PR_NOWAIT 0x0002
#define PR_ZERO 0x0008
#define PR_RWLOCK 0x0010

struct pool {
	size_t pr_size;
};

void *compat_arena_alloc(size_t size);
void compat_arena_free(void *ptr);

static inline void
pool_init(struct pool *pp, size_t size, u_int align, u_int ioff, int flags,
    const char *wchan, void *palloc)
{
	(void)align;
	(void)ioff;
	(void)flags;
	(void)wchan;
	(void)palloc;
	pp->pr_size = size;
}

static inline void
pool_setipl(struct pool *pp, int ipl)
{
	(void)pp;
	(void)ipl;
}

static inline void *
pool_get(struct pool *pp, int flags)
{
	void *p = compat_arena_alloc(pp->pr_size);
	if (p && (flags & PR_ZERO)) {
		unsigned char *b = p;
		for (size_t i = 0; i < pp->pr_size; i++)
			b[i] = 0;
	}
	return p;
}

static inline void
pool_put(struct pool *pp, void *v)
{
	(void)pp;
	compat_arena_free(v);
}

static inline void
pool_destroy(struct pool *pp)
{
	(void)pp;
}
