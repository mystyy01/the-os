#pragma once

#define READ_ONCE(x) (*(const volatile __typeof__(x) *)&(x))
#define WRITE_ONCE(x, val) (*(volatile __typeof__(x) *)&(x) = (val))

#define membar_enter() __asm volatile("mfence" ::: "memory")
#define membar_exit() __asm volatile("mfence" ::: "memory")
#define membar_producer() __asm volatile("sfence" ::: "memory")
#define membar_consumer() __asm volatile("lfence" ::: "memory")
#define membar_sync() __asm volatile("mfence" ::: "memory")

static inline unsigned int
atomic_add_int_nv(volatile unsigned int *p, int v)
{
	return __atomic_add_fetch(p, v, __ATOMIC_SEQ_CST);
}

static inline unsigned int
atomic_sub_int_nv(volatile unsigned int *p, int v)
{
	return __atomic_sub_fetch(p, v, __ATOMIC_SEQ_CST);
}

static inline void
atomic_add_int(volatile unsigned int *p, int v)
{
	__atomic_add_fetch(p, v, __ATOMIC_SEQ_CST);
}

static inline void
atomic_sub_int(volatile unsigned int *p, int v)
{
	__atomic_sub_fetch(p, v, __ATOMIC_SEQ_CST);
}

static inline void
atomic_inc_int(volatile unsigned int *p)
{
	__atomic_add_fetch(p, 1, __ATOMIC_SEQ_CST);
}

static inline void
atomic_dec_int(volatile unsigned int *p)
{
	__atomic_sub_fetch(p, 1, __ATOMIC_SEQ_CST);
}

static inline unsigned int
atomic_inc_int_nv(volatile unsigned int *p)
{
	return __atomic_add_fetch(p, 1, __ATOMIC_SEQ_CST);
}

static inline unsigned int
atomic_dec_int_nv(volatile unsigned int *p)
{
	return __atomic_sub_fetch(p, 1, __ATOMIC_SEQ_CST);
}

static inline unsigned int
atomic_cas_uint(volatile unsigned int *p, unsigned int old, unsigned int new_)
{
	__atomic_compare_exchange_n(p, &old, new_, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
	return old;
}

static inline unsigned int
atomic_swap_uint(volatile unsigned int *p, unsigned int new_)
{
	return __atomic_exchange_n(p, new_, __ATOMIC_SEQ_CST);
}

static inline void *
atomic_swap_ptr(volatile void *p, void *new_)
{
	return __atomic_exchange_n((void *volatile *)p, new_, __ATOMIC_SEQ_CST);
}

static inline void *
atomic_cas_ptr(volatile void *p, void *old, void *new_)
{
	__atomic_compare_exchange_n((void *volatile *)p, &old, new_, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
	return old;
}

static inline void
atomic_setbits_int(volatile unsigned int *p, unsigned int bits)
{
	__atomic_or_fetch(p, bits, __ATOMIC_SEQ_CST);
}

static inline void
atomic_clearbits_int(volatile unsigned int *p, unsigned int bits)
{
	__atomic_and_fetch(p, ~bits, __ATOMIC_SEQ_CST);
}
