#pragma once
#include <stdint.h>
#include <stdbool.h>

static inline void
wbinvd_on_all_cpus(void)
{
	/* privileged instruction, unavailable from ring3; no-op */
}

#define CPUIDECX_SSE41 0x00080000
#define CPUIDECX_HV 0x80000000
#define CPUID_PAT 0x00010000
#define SEFF0EBX_CLFLUSHOPT 0x00800000

struct schedstate_percpu {
	int spc_schedflags;
};

#define SPCF_SHOULDYIELD 0x0001

struct cpu_info {
	unsigned int ci_feature_flags;
	unsigned int ci_feature_sefflags_ebx;
	unsigned int ci_cflushsz;
	unsigned int ci_cpuid;
	int ci_idepth;
	int ci_inatomic;
	struct schedstate_percpu ci_schedstate;
};

static inline void
sched_pause(void (*func)(void))
{
	(void)func;
}

#define __membar(x) __asm volatile(x ::: "memory")

static inline void
yield(void)
{
	unsigned long res;
	register unsigned long r10 __asm("r10") = 0;
	__asm volatile("syscall"
		: "=a"(res)
		: "a"(9UL), "D"(0UL), "d"(0UL), "r"(r10)
		: "rcx", "r11", "memory");
}

static inline void
x86_cpuid(uint32_t leaf, uint32_t *regs)
{
	__asm volatile("cpuid"
		: "=a"(regs[0]), "=b"(regs[1]), "=c"(regs[2]), "=d"(regs[3])
		: "a"(leaf), "c"(0));
}

static inline uint32_t
cpu_ecxfeature_get(void)
{
	static bool inited = false;
	static uint32_t ecx;
	if (!inited) {
		uint32_t regs[4];
		x86_cpuid(1, regs);
		ecx = regs[2];
		inited = true;
	}
	return ecx;
}
#define cpu_ecxfeature (cpu_ecxfeature_get())

static inline int
cpu_number(void)
{
	return 0;
}

static inline struct cpu_info *
curcpu(void)
{
	static bool inited = false;
	static struct cpu_info ci;
	if (!inited) {
		uint32_t regs[4];
		x86_cpuid(1, regs);
		ci.ci_feature_flags = regs[3];
		ci.ci_cflushsz = ((regs[1] >> 8) & 0xFF) * 8;
		x86_cpuid(7, regs);
		ci.ci_feature_sefflags_ebx = regs[1];
		inited = true;
	}
	return &ci;
}
