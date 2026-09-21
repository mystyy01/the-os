#pragma once
#include <sys/stdint.h>

#define RW_WRITE 0x0001UL
#define RW_READ 0x0002UL
#define RW_DOWNGRADE 0x0004UL
#define RW_NOSLEEP 0x0040UL
#define RW_INTR 0x0080UL
#define RW_DUPOK 0x0400UL
#define RWL_NOWITNESS 0x0800UL

#define RWLOCK_WRITER 0xffffffffU
#define RWLOCK_WAIT_NS 1000000ULL

extern uint64_t os_sleep_prepare(uint64_t ident, uint64_t timeout_ns);
extern uint64_t os_sleep_commit(uint64_t handle);
extern void os_wakeup(uint64_t ident);

struct rwlock {
	volatile unsigned int state;
	volatile unsigned int waiters;
};

#define RWLOCK_INITIALIZER(name) { 0, 0 }

static inline void
rw_init(struct rwlock *l, const char *name)
{
	(void)name;
	l->state = 0;
	l->waiters = 0;
}

static inline void
rw_init_flags(struct rwlock *l, const char *name, int flags)
{
	(void)flags;
	rw_init(l, name);
}

static inline void
rw_park(struct rwlock *l)
{
	uint64_t id = (uint64_t)(uintptr_t)l;

	__atomic_add_fetch(&l->waiters, 1, __ATOMIC_ACQ_REL);
	os_sleep_commit(os_sleep_prepare(id, RWLOCK_WAIT_NS));
	__atomic_sub_fetch(&l->waiters, 1, __ATOMIC_ACQ_REL);
}

static inline void
rw_unpark(struct rwlock *l)
{
	if (__atomic_load_n(&l->waiters, __ATOMIC_ACQUIRE) != 0)
		os_wakeup((uint64_t)(uintptr_t)l);
}

static inline int
rw_try_write(struct rwlock *l)
{
	unsigned int free = 0;

	return __atomic_compare_exchange_n(&l->state, &free, RWLOCK_WRITER, 0,
	    __ATOMIC_ACQUIRE, __ATOMIC_RELAXED);
}

static inline int
rw_try_read(struct rwlock *l)
{
	unsigned int n;

	for (;;) {
		n = __atomic_load_n(&l->state, __ATOMIC_ACQUIRE);
		if (n == RWLOCK_WRITER)
			return 0;
		if (__atomic_compare_exchange_n(&l->state, &n, n + 1, 1,
		    __ATOMIC_ACQUIRE, __ATOMIC_RELAXED))
			return 1;
	}
}

static inline void
rw_enter_write(struct rwlock *l)
{
	while (!rw_try_write(l))
		rw_park(l);
}

static inline void
rw_enter_read(struct rwlock *l)
{
	while (!rw_try_read(l))
		rw_park(l);
}

static inline void
rw_exit_write(struct rwlock *l)
{
	__atomic_store_n(&l->state, 0, __ATOMIC_RELEASE);
	rw_unpark(l);
}

static inline void
rw_exit_read(struct rwlock *l)
{
	if (__atomic_sub_fetch(&l->state, 1, __ATOMIC_ACQ_REL) == 0)
		rw_unpark(l);
}

static inline void
rw_exit(struct rwlock *l)
{
	if (__atomic_load_n(&l->state, __ATOMIC_ACQUIRE) == RWLOCK_WRITER)
		rw_exit_write(l);
	else
		rw_exit_read(l);
}

static inline int
rw_enter(struct rwlock *l, int flags)
{
	if (flags & RW_NOSLEEP) {
		if (flags & RW_READ)
			return rw_try_read(l) ? 0 : 1;
		return rw_try_write(l) ? 0 : 1;
	}

	if (flags & RW_READ)
		rw_enter_read(l);
	else
		rw_enter_write(l);
	return 0;
}

static inline int
rw_status(struct rwlock *l)
{
	unsigned int n = __atomic_load_n(&l->state, __ATOMIC_ACQUIRE);

	if (n == RWLOCK_WRITER)
		return RW_WRITE;
	if (n != 0)
		return RW_READ;
	return 0;
}
