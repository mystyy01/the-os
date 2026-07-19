#pragma once

static inline int
ffs(int mask)
{
	return mask == 0 ? 0 : __builtin_ctz((unsigned int)mask) + 1;
}

static inline int
ffsl(long mask)
{
	return mask == 0 ? 0 : __builtin_ctzl((unsigned long)mask) + 1;
}

static inline int
fls(int mask)
{
	return mask == 0 ? 0 : 32 - __builtin_clz((unsigned int)mask);
}

static inline int
flsl(long mask)
{
	return mask == 0 ? 0 : 64 - __builtin_clzl((unsigned long)mask);
}
