#pragma once
#include <stddef.h>

#define M_WAITOK 0
#define M_NOWAIT 1
#define M_ZERO 0x100
#define M_CANFAIL 0x200

#define M_DRM 145
#define M_DEVBUF 2
#define M_TEMP 127

void *compat_arena_alloc(size_t size);
void compat_arena_free(void *ptr);

#define malloc(sz, type, flags) compat_arena_alloc(sz)
#define mallocarray(n, sz, type, flags) compat_arena_alloc((size_t)(n) * (size_t)(sz))

static inline void
free(void *ptr, int type, size_t sz)
{
	(void)type;
	(void)sz;
	compat_arena_free(ptr);
}
