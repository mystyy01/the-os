#pragma once
#include <stdint.h>
#include <stddef.h>
#include <sys/types.h>

typedef uint64_t bus_addr_t;
typedef uint64_t bus_size_t;
typedef int bus_space_tag_t;
typedef uint64_t bus_space_handle_t;

#define BUS_SPACE_MAP_LINEAR 0x01
#define BUS_SPACE_MAP_PREFETCHABLE 0x02

static inline void
outb(uint16_t port, uint8_t val)
{
	__asm volatile("outb %0, %1" : : "a"(val), "Nd"(port));
}

static inline uint8_t
inb(uint16_t port)
{
	uint8_t v;
	__asm volatile("inb %1, %0" : "=a"(v) : "Nd"(port));
	return v;
}

static inline uint8_t
bus_space_read_1(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off)
{
	(void)t;
	return *(volatile uint8_t *)(h + off);
}

static inline uint16_t
bus_space_read_2(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off)
{
	(void)t;
	return *(volatile uint16_t *)(h + off);
}

static inline uint32_t
bus_space_read_4(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off)
{
	(void)t;
	return *(volatile uint32_t *)(h + off);
}

static inline uint64_t
bus_space_read_8(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off)
{
	(void)t;
	return *(volatile uint64_t *)(h + off);
}

static inline void
bus_space_write_1(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off, uint8_t v)
{
	(void)t;
	*(volatile uint8_t *)(h + off) = v;
}

static inline void
bus_space_write_2(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off, uint16_t v)
{
	(void)t;
	*(volatile uint16_t *)(h + off) = v;
}

static inline void
bus_space_write_4(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off, uint32_t v)
{
	(void)t;
	*(volatile uint32_t *)(h + off) = v;
}

static inline void
bus_space_write_8(bus_space_tag_t t, bus_space_handle_t h, bus_size_t off, uint64_t v)
{
	(void)t;
	*(volatile uint64_t *)(h + off) = v;
}

static inline int
bus_space_map(bus_space_tag_t t, bus_addr_t addr, bus_size_t size, int flags, bus_space_handle_t *hp)
{
	(void)t;
	(void)size;
	(void)flags;
	*hp = addr;
	return 0;
}

static inline void
bus_space_unmap(bus_space_tag_t t, bus_space_handle_t h, bus_size_t size)
{
	(void)t;
	(void)h;
	(void)size;
}

static inline void *
bus_space_vaddr(bus_space_tag_t t, bus_space_handle_t h)
{
	(void)t;
	return (void *)h;
}

typedef struct bus_dma_segment {
	bus_addr_t ds_addr;
	bus_size_t ds_len;
} bus_dma_segment_t;

typedef struct bus_dmamap {
	bus_size_t dm_mapsize;
	int dm_nsegs;
	bus_dma_segment_t dm_segs[1];
} *bus_dmamap_t;

typedef int bus_dma_tag_t;

#define BUS_DMA_NOWAIT 0x01
#define BUS_DMA_ALLOCNOW 0x02
#define BUS_DMA_ZERO 0x04
#define BUS_DMA_COHERENT 0x08
#define BUS_DMA_WAITOK 0x10

static inline int
bus_dmamap_load_raw(bus_dma_tag_t t, bus_dmamap_t map,
    bus_dma_segment_t *segs, int nsegs, bus_size_t size, int flags)
{
	(void)t;
	(void)map;
	(void)segs;
	(void)nsegs;
	(void)size;
	(void)flags;
	return -1;
}

static inline int
bus_dmamap_create(bus_dma_tag_t t, bus_size_t size, int nsegments,
    bus_size_t maxsegsz, bus_size_t boundary, int flags, bus_dmamap_t *mapp)
{
	(void)t;
	(void)size;
	(void)nsegments;
	(void)maxsegsz;
	(void)boundary;
	(void)flags;
	(void)mapp;
	return -1;
}

static inline void
bus_dmamap_destroy(bus_dma_tag_t t, bus_dmamap_t map)
{
	(void)t;
	(void)map;
}

static inline int
bus_dmamem_alloc(bus_dma_tag_t t, bus_size_t size, bus_size_t alignment,
    bus_size_t boundary, bus_dma_segment_t *segs, int nsegs, int *rsegs,
    int flags)
{
	(void)t;
	(void)size;
	(void)alignment;
	(void)boundary;
	(void)segs;
	(void)nsegs;
	(void)rsegs;
	(void)flags;
	return -1;
}

static inline void
bus_dmamem_free(bus_dma_tag_t t, bus_dma_segment_t *segs, int nsegs)
{
	(void)t;
	(void)segs;
	(void)nsegs;
}

static inline int
bus_dmamem_map(bus_dma_tag_t t, bus_dma_segment_t *segs, int nsegs,
    size_t size, caddr_t *kvap, int flags)
{
	(void)t;
	(void)segs;
	(void)nsegs;
	(void)size;
	(void)kvap;
	(void)flags;
	return -1;
}

static inline void
bus_dmamem_unmap(bus_dma_tag_t t, caddr_t kva, size_t size)
{
	(void)t;
	(void)kva;
	(void)size;
}

static inline int
bus_dmamap_load(bus_dma_tag_t t, bus_dmamap_t map, void *buf, size_t buflen,
    void *p, int flags)
{
	(void)t;
	(void)map;
	(void)buf;
	(void)buflen;
	(void)p;
	(void)flags;
	return -1;
}

static inline void
bus_dmamap_unload(bus_dma_tag_t t, bus_dmamap_t map)
{
	(void)t;
	(void)map;
}
