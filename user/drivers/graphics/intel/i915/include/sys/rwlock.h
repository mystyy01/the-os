#pragma once

#define RW_WRITE 0x0001UL
#define RW_READ 0x0002UL
#define RW_DOWNGRADE 0x0004UL
#define RW_NOSLEEP 0x0040UL
#define RW_INTR 0x0080UL
#define RW_DUPOK 0x0400UL
#define RWL_NOWITNESS 0x0800UL

struct rwlock {
	int dummy;
};

#define RWLOCK_INITIALIZER(name) { 0 }

static inline void
rw_init(struct rwlock *l, const char *name)
{
	(void)name;
	l->dummy = 0;
}

static inline void
rw_init_flags(struct rwlock *l, const char *name, int flags)
{
	(void)name;
	(void)flags;
	l->dummy = 0;
}

static inline int
rw_enter(struct rwlock *l, int flags)
{
	(void)l;
	(void)flags;
	return 0;
}

static inline void
rw_enter_read(struct rwlock *l)
{
	(void)l;
}

static inline void
rw_enter_write(struct rwlock *l)
{
	(void)l;
}

static inline void
rw_exit(struct rwlock *l)
{
	(void)l;
}

static inline void
rw_exit_read(struct rwlock *l)
{
	(void)l;
}

static inline void
rw_exit_write(struct rwlock *l)
{
	(void)l;
}

static inline int
rw_status(struct rwlock *l)
{
	(void)l;
	return 0;
}
