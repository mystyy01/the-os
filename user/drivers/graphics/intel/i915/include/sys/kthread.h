#pragma once

struct proc;

static inline int
kthread_create(void (*func)(void *), void *arg, struct proc **newpp,
    const char *name, ...)
{
	(void)func;
	(void)arg;
	(void)name;
	if (newpp)
		*newpp = 0;
	return -1;
}

static inline void
kthread_exit(int ret)
{
	(void)ret;
	for (;;)
		;
}
