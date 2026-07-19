#pragma once
#include <sys/rwlock.h>

struct mutex {
	int dummy;
};

#define MUTEX_INITIALIZER(ipl) { 0 }

static inline void
mtx_init(struct mutex *m, int ipl)
{
	(void)ipl;
	m->dummy = 0;
}

#define MTX_NOWITNESS 0x01

static inline void
mtx_init_flags(struct mutex *m, int ipl, const char *name, int flags)
{
	(void)ipl;
	(void)name;
	(void)flags;
	m->dummy = 0;
}

static inline void
mtx_enter(struct mutex *m)
{
	(void)m;
}

static inline void
mtx_leave(struct mutex *m)
{
	(void)m;
}

static inline int
mtx_enter_try(struct mutex *m)
{
	(void)m;
	return 1;
}

#define MUTEX_ASSERT_LOCKED(m) ((void)(m))
#define MUTEX_ASSERT_UNLOCKED(m) ((void)(m))
