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

extern void i915_timeout_add_ns(void *owner, void (*func)(void *), void *arg, uint64_t delay_ns);
extern int i915_timeout_del(void *owner);

static inline int
timeout_add(struct timeout *to, int ticks)
{
	uint64_t ns = (ticks > 0) ? ((uint64_t)ticks * 10000000ULL) : 0;
	to->to_pending = 1;
	i915_timeout_add_ns(to, to->to_func, to->to_arg, ns);
	return 1;
}

static inline int
timeout_add_msec(struct timeout *to, int msec)
{
	to->to_pending = 1;
	i915_timeout_add_ns(to, to->to_func, to->to_arg, (uint64_t)msec * 1000000ULL);
	return 1;
}

static inline int
timeout_add_sec(struct timeout *to, int sec)
{
	to->to_pending = 1;
	i915_timeout_add_ns(to, to->to_func, to->to_arg, (uint64_t)sec * 1000000000ULL);
	return 1;
}

static inline int
timeout_add_nsec(struct timeout *to, uint64_t nsec)
{
	to->to_pending = 1;
	i915_timeout_add_ns(to, to->to_func, to->to_arg, nsec);
	return 1;
}

static inline int
timeout_abs_ts(struct timeout *to, const struct timespec *ts)
{
	(void)ts;
	to->to_pending = 1;
	i915_timeout_add_ns(to, to->to_func, to->to_arg, 0);
	return 1;
}

static inline int
timeout_del(struct timeout *to)
{
	to->to_pending = 0;
	return i915_timeout_del(to);
}

static inline int
timeout_del_barrier(struct timeout *to)
{
	to->to_pending = 0;
	return i915_timeout_del(to);
}

#define timeout_pending(to) ((to)->to_pending)
