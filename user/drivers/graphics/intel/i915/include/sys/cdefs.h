#pragma once

#define __aligned(x) __attribute__((__aligned__(x)))
#define __packed __attribute__((__packed__))
#define __unused __attribute__((__unused__))
#define __maybe_unused __attribute__((__unused__))
#define __always_unused __attribute__((__unused__))
#define __printflike(a, b) __attribute__((__format__(__printf__, a, b)))
#define __dead __attribute__((__noreturn__))
#define __noreturn __attribute__((__noreturn__))
#define __unreachable() __builtin_unreachable()
#define nitems(x) (sizeof(x) / sizeof((x)[0]))
#define CTASSERT(x) extern char _ctassert_[(x) ? 1 : -1] __attribute__((__unused__))

static inline int
abs(int j)
{
	return j < 0 ? -j : j;
}
