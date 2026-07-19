#pragma once
#include <stdint.h>
#include <sys/time.h>

struct timeout {
	void (*to_func)(void *);
	void *to_arg;
	int to_pending;
	int64_t to_time;
};

#define TIMEOUT_INITIALIZER(func, arg) { func, arg, 0 }

static inline void
timeout_set(struct timeout *to, void (*func)(void *), void *arg)
{
	to->to_func = func;
	to->to_arg = arg;
	to->to_pending = 0;
}

static inline int
timeout_add(struct timeout *to, int ticks)
{
	(void)ticks;
	to->to_pending = 0;
	to->to_func(to->to_arg);
	return 1;
}

static inline int
timeout_add_msec(struct timeout *to, int msec)
{
	return timeout_add(to, msec);
}

static inline int
timeout_add_sec(struct timeout *to, int sec)
{
	return timeout_add(to, sec);
}

static inline int
timeout_add_nsec(struct timeout *to, uint64_t nsec)
{
	(void)nsec;
	return timeout_add(to, 0);
}

static inline int
timeout_abs_ts(struct timeout *to, const struct timespec *ts)
{
	(void)ts;
	return timeout_add(to, 0);
}

static inline int
timeout_del(struct timeout *to)
{
	(void)to;
	return 0;
}

static inline int
timeout_del_barrier(struct timeout *to)
{
	(void)to;
	return 0;
}

#define timeout_pending(to) ((to)->to_pending)
