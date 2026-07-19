#pragma once
#define IPL_NONE 0
#define IPL_TTY 1
#define IPL_BIO 2
#define IPL_NET 3
#define IPL_VM 4
#define IPL_HIGH 5
#define IPL_MPFLOOR IPL_TTY

static inline unsigned long
intr_disable(void)
{
	return 0;
}

static inline void
intr_enable(void)
{
}

static inline void
intr_restore(unsigned long s)
{
	(void)s;
}

static inline void
intr_barrier(void *ih)
{
	(void)ih;
}

static inline int
spltty(void)
{
	return 0;
}

static inline int
splhigh(void)
{
	return 0;
}

static inline void
splx(int s)
{
	(void)s;
}
