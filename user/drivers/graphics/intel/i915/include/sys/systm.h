#pragma once
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <limits.h>
#include <string.h>
#include <sys/proc.h>
#include <errno.h>
#include <lib/libkern/libkern.h>
#include <acpi.h>
#include <machine/pmap.h>
#include <sys/uio.h>
#include <linux/iosys-map.h>
#include <sys/device.h>

int copyin(const void *uaddr, void *kaddr, size_t len);
int copyout(const void *kaddr, void *uaddr, size_t len);

extern int cold;
struct proc;
extern struct proc *curproc;

#define PCATCH 0x100
#define INFSLP UINT64_MAX

void wakeup(const volatile void *ident);
void wakeup_one(const volatile void *ident);
int msleep_nsec(const volatile void *ident, void *lock, int priority,
    const char *wmesg, uint64_t nsecs);
int msleep(const volatile void *ident, void *lock, int priority,
    const char *wmesg, int timo);

void arc4random_buf(void *buf, size_t n);
uint32_t arc4random(void);
uint32_t arc4random_uniform(uint32_t upper_bound);
int sleep_finish(uint64_t nsecs, int do_sleep);

static inline void
assertwaitok(void)
{
}

static inline int
loadfirmware(const char *name, unsigned char **bufp, size_t *buflen)
{
	(void)name;
	(void)bufp;
	(void)buflen;
	return ENOENT;
}

int printf(const char *fmt, ...);
int vprintf(const char *fmt, va_list ap);
int snprintf(char *buf, size_t size, const char *fmt, ...);
int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
void panic(const char *fmt, ...) __attribute__((__noreturn__));
void DELAY(int usec);
int tsleep(void *ident, int priority, const char *wmesg, int timo);
int tsleep_nsec(void *ident, int priority, const char *wmesg, uint64_t nsecs);

#define KASSERT(x) do { if (!(x)) panic("assert: %s", #x); } while (0)
