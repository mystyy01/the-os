#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <sys/proc.h>
#include <uvm/uvm_extern.h>
#include <sys/task.h>
#include <sys/time.h>
#include <dev/pci/pcivar.h>
#include <uvm/uvm_extern.h>
#include <dev/wscons/wsconsio.h>
#include <acpi.h>
#include <linux/workqueue.h>
#include <dev/wscons/wsdisplayvar.h>

extern uint64_t os_alloc_dma(uint64_t pages, uint64_t *phys_out);
extern void os_print(const char *s);
uint64_t os_now_ns(void);
void i915_refresh_jiffies(void);
extern uint64_t os_msi_take(void);

#define MAX_VM_PAGES 65536
static struct vm_page vm_page_table[MAX_VM_PAGES];
static size_t vm_page_count = 0;

static int (*i915_irq_func)(void *);
static void *i915_irq_arg;

void
i915_irq_register(int (*func)(void *), void *arg)
{
	i915_irq_func = func;
	i915_irq_arg = arg;
}

uint64_t
i915_irq_dispatch(void)
{
	uint64_t count = os_msi_take();
	if (count && i915_irq_func) {
		i915_irq_func(i915_irq_arg);
	}
	return count;
}

struct uvmexp_s uvmexp = { .free = 0, .npages = 0 };

void
i915_shim_set_ram_pages(uint64_t npages)
{
	uvmexp.npages = npages;
	uvmexp.free = npages;
}

#define DMA_CHUNK_PAGES 512

struct vm_page *
uvm_pagealloc(size_t npages)
{
	if (vm_page_count + npages > MAX_VM_PAGES) {
		os_print("i915: uvm_pagealloc OOM (vm_page_table full)\n");
		return (void *)0;
	}

	struct vm_page *first = &vm_page_table[vm_page_count];
	size_t done = 0;
	while (done < npages) {
		size_t chunk = npages - done;
		if (chunk > DMA_CHUNK_PAGES)
			chunk = DMA_CHUNK_PAGES;

		uint64_t phys = 0;
		uint64_t virt = os_alloc_dma(chunk, &phys);
		if (virt == 0 || phys == 0) {
			os_print("i915: uvm_pagealloc DMA alloc failed\n");
			return (void *)0;
		}

		for (size_t i = 0; i < chunk; i++) {
			vm_page_table[vm_page_count].phys_addr = phys + i * PAGE_SIZE;
			vm_page_table[vm_page_count].virt_addr = (void *)(virt + i * PAGE_SIZE);
			vm_page_count++;
		}
		done += chunk;
	}
	return first;
}

int
uvm_pglistalloc(uint64_t size, uint64_t low, uint64_t high, uint64_t alignment,
    uint64_t boundary, struct pglist *rlist, int nsegs, int waitok)
{
	(void)low;
	(void)high;
	(void)alignment;
	(void)boundary;
	(void)nsegs;
	(void)waitok;

	size_t npages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
	if (npages == 0)
		npages = 1;

	struct vm_page *pg = uvm_pagealloc(npages);
	if (pg == (void *)0)
		return -1;

	for (size_t i = 0; i < npages; i++)
		TAILQ_INSERT_TAIL(rlist, &pg[i], pageq);
	return 0;
}

void
uvm_pglistfree(struct pglist *list)
{
	(void)list;
}

struct vm_page *
uvm_atopg(uint64_t kva)
{
	for (size_t i = 0; i < vm_page_count; i++) {
		if ((uint64_t)vm_page_table[i].virt_addr == kva)
			return &vm_page_table[i];
	}
	return (void *)0;
}

uint64_t
vm_page_to_phys(struct vm_page *pg)
{
	return pg->phys_addr;
}

void *
vm_page_to_virt(struct vm_page *pg)
{
	return pg->virt_addr;
}

#define MAX_PHYSLOAD 4
struct physload_range {
	uint64_t start_pfn;
	uint64_t end_pfn;
	struct vm_page *pages;
};
static struct physload_range physload_ranges[MAX_PHYSLOAD];
static int physload_count = 0;

extern void *compat_arena_alloc(size_t size);

void
uvm_page_physload(uint64_t start, uint64_t end, uint64_t avail_start,
    uint64_t avail_end, int flags)
{
	(void)avail_start;
	(void)avail_end;
	(void)flags;

	if (physload_count >= MAX_PHYSLOAD || end <= start)
		return;

	uint64_t npages = end - start;
	struct vm_page *pages =
	    compat_arena_alloc(npages * sizeof(struct vm_page));
	if (pages == (void *)0)
		return;

	for (uint64_t i = 0; i < npages; i++) {
		pages[i].phys_addr = (start + i) * PAGE_SIZE;
		pages[i].virt_addr = (void *)0;
		pages[i].pg_flags = 0;
	}

	physload_ranges[physload_count].start_pfn = start;
	physload_ranges[physload_count].end_pfn = end;
	physload_ranges[physload_count].pages = pages;
	physload_count++;
}

struct vm_page *
phys_to_vm_page(uint64_t pa)
{
	uint64_t pfn = pa / PAGE_SIZE;
	for (int r = 0; r < physload_count; r++) {
		if (pfn >= physload_ranges[r].start_pfn &&
		    pfn < physload_ranges[r].end_pfn)
			return &physload_ranges[r].pages[pfn -
			    physload_ranges[r].start_pfn];
	}
	for (size_t i = 0; i < vm_page_count; i++) {
		if (vm_page_table[i].phys_addr == pa)
			return &vm_page_table[i];
	}
	return (void *)0;
}

static inline void
outl_port(uint16_t port, uint32_t val)
{
	__asm volatile("outl %0, %1" : : "a"(val), "Nd"(port));
}

static inline uint32_t
inl_port(uint16_t port)
{
	uint32_t v;
	__asm volatile("inl %1, %0" : "=a"(v) : "Nd"(port));
	return v;
}

uint32_t
pci_conf_read(pci_chipset_tag_t pc, pcitag_t tag, int reg)
{
	(void)pc;
	outl_port(0xCF8, 0x80000000u | tag | (uint32_t)(reg & 0xFC));
	return inl_port(0xCFC);
}

void
pci_conf_write(pci_chipset_tag_t pc, pcitag_t tag, int reg, uint32_t data)
{
	(void)pc;
	outl_port(0xCF8, 0x80000000u | tag | (uint32_t)(reg & 0xFC));
	outl_port(0xCFC, data);
}

int
pci_get_capability(pci_chipset_tag_t pc, pcitag_t tag, int cap, int *offset, uint32_t *value)
{
	uint32_t status = pci_conf_read(pc, tag, 0x04);
	if (!(status & 0x00100000))
		return 0;

	int ptr = (int)(pci_conf_read(pc, tag, 0x34) & 0xFF);
	int guard = 0;
	while (ptr != 0 && guard++ < 64) {
		uint32_t capreg = pci_conf_read(pc, tag, ptr);
		if ((int)(capreg & 0xFF) == cap) {
			if (offset)
				*offset = ptr;
			if (value)
				*value = capreg;
			return 1;
		}
		ptr = (int)((capreg >> 8) & 0xFF);
	}
	return 0;
}

void *compat_arena_alloc(size_t size);
void compat_arena_free(void *ptr);

int
copyin(const void *uaddr, void *kaddr, size_t len)
{
	memcpy(kaddr, uaddr, len);
	return 0;
}

int
copyout(const void *kaddr, void *uaddr, size_t len)
{
	memcpy(uaddr, kaddr, len);
	return 0;
}

static void
putc_buf(char *buf, size_t size, size_t *pos, char c)
{
	if (*pos < size)
		buf[*pos] = c;
	(*pos)++;
}

static void
puts_buf(char *buf, size_t size, size_t *pos, const char *s)
{
	while (*s)
		putc_buf(buf, size, pos, *s++);
}

static void
putnum_buf(char *buf, size_t size, size_t *pos, unsigned long long v, int base, int upper, int is_signed, long long sv)
{
	char tmp[32];
	int n = 0;
	const char *digits = upper ? "0123456789ABCDEF" : "0123456789abcdef";
	unsigned long long uv;

	if (is_signed && sv < 0) {
		putc_buf(buf, size, pos, '-');
		uv = (unsigned long long)(-sv);
	} else if (is_signed) {
		uv = (unsigned long long)sv;
	} else {
		uv = v;
	}

	if (uv == 0) {
		tmp[n++] = '0';
	} else {
		while (uv > 0) {
			tmp[n++] = digits[uv % base];
			uv /= base;
		}
	}
	while (n > 0)
		putc_buf(buf, size, pos, tmp[--n]);
}

int
vsnprintf(char *buf, size_t size, const char *fmt, va_list ap)
{
	size_t pos = 0;
	const char *p = fmt;

	while (*p) {
		if (*p != '%') {
			putc_buf(buf, size, &pos, *p++);
			continue;
		}
		p++;

		while (*p == '-' || *p == '+' || *p == ' ' || *p == '#' || *p == '0')
			p++;
		int width = 0;
		while (*p >= '0' && *p <= '9') {
			width = width * 10 + (*p - '0');
			p++;
		}
		if (*p == '.') {
			p++;
			while (*p >= '0' && *p <= '9')
				p++;
		}

		int longcount = 0;
		int is_size_t = 0;
		for (;;) {
			if (*p == 'l') {
				longcount++;
				p++;
			} else if (*p == 'z' || *p == 'j' || *p == 't') {
				is_size_t = 1;
				p++;
			} else if (*p == 'h') {
				p++;
			} else {
				break;
			}
		}
		int wide = longcount >= 1 || is_size_t;

		switch (*p) {
		case 's': {
			const char *s = va_arg(ap, const char *);
			puts_buf(buf, size, &pos, s ? s : "(null)");
			break;
		}
		case 'c': {
			char c = (char)va_arg(ap, int);
			putc_buf(buf, size, &pos, c);
			break;
		}
		case 'd':
		case 'i': {
			long long v = wide ? va_arg(ap, long) : va_arg(ap, int);
			putnum_buf(buf, size, &pos, 0, 10, 0, 1, v);
			break;
		}
		case 'u': {
			unsigned long long v = wide ? va_arg(ap, unsigned long) : va_arg(ap, unsigned int);
			putnum_buf(buf, size, &pos, v, 10, 0, 0, 0);
			break;
		}
		case 'x': {
			unsigned long long v = wide ? va_arg(ap, unsigned long) : va_arg(ap, unsigned int);
			putnum_buf(buf, size, &pos, v, 16, 0, 0, 0);
			break;
		}
		case 'X': {
			unsigned long long v = wide ? va_arg(ap, unsigned long) : va_arg(ap, unsigned int);
			putnum_buf(buf, size, &pos, v, 16, 1, 0, 0);
			break;
		}
		case 'p': {
			void *v = va_arg(ap, void *);
			puts_buf(buf, size, &pos, "0x");
			putnum_buf(buf, size, &pos, (unsigned long long)(uintptr_t)v, 16, 0, 0, 0);
			break;
		}
		case '%':
			putc_buf(buf, size, &pos, '%');
			break;
		default:
			putc_buf(buf, size, &pos, '%');
			if (*p)
				putc_buf(buf, size, &pos, *p);
			break;
		}
		(void)width;
		p++;
	}

	if (size > 0) {
		size_t end = pos < size ? pos : size - 1;
		buf[end] = 0;
	}
	return (int)pos;
}

int
snprintf(char *buf, size_t size, const char *fmt, ...)
{
	va_list ap;
	int r;
	va_start(ap, fmt);
	r = vsnprintf(buf, size, fmt, ap);
	va_end(ap);
	return r;
}

extern void os_print(const char *s);

int
vprintf(const char *fmt, va_list ap)
{
	char buf[256];
	int r = vsnprintf(buf, sizeof(buf), fmt, ap);
	os_print(buf);
	return r;
}

int
printf(const char *fmt, ...)
{
	va_list ap;
	int r;
	va_start(ap, fmt);
	r = vprintf(fmt, ap);
	va_end(ap);
	return r;
}

void
panic(const char *fmt, ...)
{
	(void)fmt;
	for (;;)
		;
}

void
DELAY(int usec)
{
	volatile long i;
	for (i = 0; i < (long)usec * 1000; i++)
		;
}

extern uint64_t os_sleep(uint64_t ident, uint64_t timeout_ns);
extern void os_wakeup(uint64_t ident);

int
tsleep(void *ident, int priority, const char *wmesg, int timo)
{
	(void)priority;
	(void)wmesg;
	uint64_t ns = (timo > 0) ? ((uint64_t)timo * 10000000ULL) : 0;
	uint64_t r = os_sleep((uint64_t)(uintptr_t)ident, ns);
	i915_refresh_jiffies();
	return (r == 1) ? 1 : 0;
}

int
tsleep_nsec(void *ident, int priority, const char *wmesg, uint64_t nsecs)
{
	(void)priority;
	(void)wmesg;
	uint64_t ns = (nsecs == UINT64_MAX) ? 0 : nsecs;
	uint64_t r = os_sleep((uint64_t)(uintptr_t)ident, ns);
	i915_refresh_jiffies();
	return (r == 1) ? 1 : 0;
}

int cold = 0;
int nowake = 0;
int hz = 100;
volatile unsigned long ticks = 0;

static uint64_t
rdtsc_now(void)
{
	uint32_t lo, hi;
	__asm volatile("lfence; rdtsc" : "=a"(lo), "=d"(hi));
	return ((uint64_t)hi << 32) | lo;
}

#define ASSUMED_TSC_HZ 3000000000ULL

void
nanouptime(struct timespec *ts)
{
	uint64_t ns = rdtsc_now() / (ASSUMED_TSC_HZ / 1000000000ULL);
	ts->tv_sec = (long)(ns / 1000000000ULL);
	ts->tv_nsec = (long)(ns % 1000000000ULL);
}

uint64_t
os_now_ns(void)
{
	return rdtsc_now() / (ASSUMED_TSC_HZ / 1000000000ULL);
}

void
microuptime(struct timeval *tv)
{
	uint64_t us = rdtsc_now() / (ASSUMED_TSC_HZ / 1000000ULL);
	tv->tv_sec = (long)(us / 1000000ULL);
	tv->tv_usec = (long)(us % 1000000ULL);
}

void
getnanouptime(struct timespec *ts)
{
	nanouptime(ts);
}

void
nanotime(struct timespec *ts)
{
	nanouptime(ts);
}

void
getnanotime(struct timespec *ts)
{
	nanouptime(ts);
}

time_t
gettime(void)
{
	struct timespec ts;
	nanouptime(&ts);
	return ts.tv_sec;
}

volatile int tick = 10000;

void
delay(unsigned int usecs)
{
	DELAY((int)usecs);
}

static struct process dummy_process = { .ps_pid = 6, .ps_comm = "i915" };
static struct proc dummy_curproc = { .p_p = &dummy_process };
struct proc *curproc = &dummy_curproc;

int
suser(struct proc *p)
{
	(void)p;
	return 0;
}

static uint64_t rng_state = 0;

uint32_t
arc4random(void)
{
	if (rng_state == 0)
		rng_state = rdtsc_now() | 1;
	rng_state ^= rng_state << 13;
	rng_state ^= rng_state >> 7;
	rng_state ^= rng_state << 17;
	return (uint32_t)rng_state;
}

uint32_t
arc4random_uniform(uint32_t upper_bound)
{
	if (upper_bound < 2)
		return 0;
	return arc4random() % upper_bound;
}

void
arc4random_buf(void *buf, size_t n)
{
	unsigned char *p = buf;
	while (n > 0) {
		uint32_t v = arc4random();
		size_t chunk = n < 4 ? n : 4;
		for (size_t i = 0; i < chunk; i++)
			p[i] = (unsigned char)(v >> (i * 8));
		p += chunk;
		n -= chunk;
	}
}

void
wakeup(const volatile void *ident)
{
	os_wakeup((uint64_t)(uintptr_t)ident);
}

void
wakeup_one(const volatile void *ident)
{
	os_wakeup((uint64_t)(uintptr_t)ident);
}

int
msleep_nsec(const volatile void *ident, void *lock, int priority, const char *wmesg, uint64_t nsecs)
{
	(void)lock;
	(void)priority;
	(void)wmesg;
	uint64_t ns = (nsecs == UINT64_MAX) ? 0 : nsecs;
	uint64_t r = os_sleep((uint64_t)(uintptr_t)ident, ns);
	i915_refresh_jiffies();
	return (r == 1) ? 1 : 0;
}

int
msleep(const volatile void *ident, void *lock, int priority, const char *wmesg, int timo)
{
	(void)lock;
	(void)priority;
	(void)wmesg;
	uint64_t ns = (timo > 0) ? ((uint64_t)timo * 10000000ULL) : 0;
	uint64_t r = os_sleep((uint64_t)(uintptr_t)ident, ns);
	i915_refresh_jiffies();
	return (r == 1) ? 1 : 0;
}

struct kmem_va_mode kv_page;
struct kmem_va_mode kv_any;
struct kmem_pa_mode kp_dirty;
struct kmem_pa_mode kp_zero;
struct kmem_pa_mode kp_none;
struct kmem_dyn_mode kd_nowait;
struct kmem_dyn_mode kd_waitok;
struct vm_map *phys_map = 0;
struct vm_map *kernel_map = 0;

void *
km_alloc(size_t size, const struct kmem_va_mode *kv, const struct kmem_pa_mode *kp,
    const struct kmem_dyn_mode *kd)
{
	(void)kv;
	(void)kp;
	(void)kd;
	return compat_arena_alloc(size);
}

void
km_free(void *addr, size_t size, const struct kmem_va_mode *kv, const struct kmem_pa_mode *kp)
{
	(void)size;
	(void)kv;
	(void)kp;
	compat_arena_free(addr);
}

static struct taskq dummy_taskq;

struct taskq *
taskq_create(const char *name, int nthreads, int ipl, int flags)
{
	(void)name;
	(void)nthreads;
	(void)ipl;
	(void)flags;
	return &dummy_taskq;
}

void
taskq_destroy(struct taskq *tq)
{
	(void)tq;
}

struct taskq *systq = &dummy_taskq;

volatile unsigned long jiffies = 0;

void
i915_refresh_jiffies(void)
{
	jiffies = (unsigned long)(os_now_ns() / 10000000ULL);
	ticks = jiffies;
}

int vga_console_attached = 0;
int cpuspeed = 2000;
unsigned long physmem = 1UL << 20;

void
pmap_zero_page(struct vm_page *pg)
{
	memset(pg->virt_addr, 0, PAGE_SIZE);
}

int
sleep_finish(uint64_t nsecs, int do_sleep)
{
	if (do_sleep) {
		uint64_t ns = (nsecs == UINT64_MAX) ? 0 : nsecs;
		os_sleep((uint64_t)(uintptr_t)curproc, ns);
	}
	i915_refresh_jiffies();
	return 0;
}
char *hw_vendor = 0;
char *hw_prod = 0;
char *hw_ver = 0;
char osrelease[] = "the-os";
char machine[] = "amd64";

struct uvm_constraint_range no_constraint = { 0, ~0ULL };
struct uvm_constraint_range dma_constraint = { 0, ~0ULL };

int
wsdisplay_cnattach(const struct wsscreen_descr *type, void *cookie,
    int ccol, int crow, uint32_t defattr)
{
	(void)type;
	(void)cookie;
	(void)ccol;
	(void)crow;
	(void)defattr;
	return 0;
}

int (*ws_get_param)(struct wsdisplay_param *) = 0;
int (*ws_set_param)(struct wsdisplay_param *) = 0;

struct acpi_softc *acpi_softc = 0;
