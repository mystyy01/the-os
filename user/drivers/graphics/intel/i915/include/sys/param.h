#pragma once
#include <machine/cpu.h>

#define roundup(x, y) ((((x) + ((y) - 1)) / (y)) * (y))
#define rounddown(x, y) (((x) / (y)) * (y))
#define howmany(x, y) (((x) + ((y) - 1)) / (y))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define NBBY 8

#define CPU_BUSY_CYCLE() __asm volatile("pause" ::: "memory")

extern volatile int tick;
void delay(unsigned int usecs);

#define VM_MIN_ADDRESS 0UL
#define VM_MAXUSER_ADDRESS (~0UL)
#define MAXCOMLEN 24

extern int nowake;
#define PWAIT 0x02
#define PZERO 22
#define MSEC_TO_NSEC(ms) ((uint64_t)(ms) * 1000000ULL)

#define CLONE_SHIFT 8
#define minor(x) ((x) & 0xFF)
#define major(x) (((x) >> 8) & 0xFF)
