#pragma once
#include <stdint.h>

#define LITTLE_ENDIAN 1234
#define BIG_ENDIAN 4321
#define BYTE_ORDER LITTLE_ENDIAN

#define htole16(x) ((uint16_t)(x))
#define htole32(x) ((uint32_t)(x))
#define htole64(x) ((uint64_t)(x))
#define letoh16(x) ((uint16_t)(x))
#define letoh32(x) ((uint32_t)(x))
#define letoh64(x) ((uint64_t)(x))

#define htobe16(x) ((uint16_t)__builtin_bswap16((uint16_t)(x)))
#define htobe32(x) ((uint32_t)__builtin_bswap32((uint32_t)(x)))
#define htobe64(x) ((uint64_t)__builtin_bswap64((uint64_t)(x)))
#define betoh16(x) htobe16(x)
#define betoh32(x) htobe32(x)
#define betoh64(x) htobe64(x)

static inline void
htolem16(volatile void *p, uint16_t v)
{
	*(volatile uint16_t *)p = htole16(v);
}
static inline void
htolem32(volatile void *p, uint32_t v)
{
	*(volatile uint32_t *)p = htole32(v);
}
static inline void
htolem64(volatile void *p, uint64_t v)
{
	*(volatile uint64_t *)p = htole64(v);
}
static inline uint16_t
lemtoh16(const volatile void *p)
{
	return letoh16(*(const volatile uint16_t *)p);
}
static inline uint32_t
lemtoh32(const volatile void *p)
{
	return letoh32(*(const volatile uint32_t *)p);
}
static inline uint64_t
lemtoh64(const volatile void *p)
{
	return letoh64(*(const volatile uint64_t *)p);
}
