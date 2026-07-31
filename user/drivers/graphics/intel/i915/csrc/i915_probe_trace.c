#include <stdint.h>

#include "gt/uc/intel_guc.h"
#include "gt/intel_gt.h"
#include "gt/intel_gtt.h"
#include "gem/i915_gem_object.h"
#include "i915_request.h"

extern void os_print(const char *s);

static struct intel_guc_ct *trace_ct;

static void
trace_enter(const char *name)
{
	os_print(name);
}

static void
trace_ret(const char *name, int ret)
{
	char buf[80];
	int i = 0;
	int neg = (ret < 0);
	if (neg) {
		buf[i++] = 0x1b;
		buf[i++] = '[';
		buf[i++] = '3';
		buf[i++] = '1';
		buf[i++] = 'm';
	}
	int n = 0;
	while (name[n] && n < 40) {
		buf[i++] = name[n];
		n++;
	}
	buf[i++] = '=';
	if (ret < 0) {
		buf[i++] = '-';
		ret = -ret;
	}
	char digits[12];
	int d = 0;
	if (ret == 0)
		digits[d++] = '0';
	while (ret > 0 && d < 11) {
		digits[d++] = '0' + (ret % 10);
		ret /= 10;
	}
	while (d > 0)
		buf[i++] = digits[--d];
	if (neg) {
		buf[i++] = 0x1b;
		buf[i++] = '[';
		buf[i++] = '0';
		buf[i++] = 'm';
	}
	buf[i] = 0;
	os_print(buf);
}

static void
trace_ptr(const char *name, uint64_t v)
{
	char buf[80];
	int i = 0;
	while (name[i] && i < 48) {
		buf[i] = name[i];
		i++;
	}
	buf[i++] = '0';
	buf[i++] = 'x';
	for (int s = 60; s >= 0; s -= 4) {
		int nib = (v >> s) & 0xf;
		buf[i++] = nib < 10 ? '0' + nib : 'a' + (nib - 10);
	}
	buf[i] = 0;
	os_print(buf);
}

static uint32_t
trace_sum(const uint8_t *p, uint32_t n)
{
	uint32_t sum = 0;
	for (uint32_t i = 0; i < n; i++)
		sum += p[i];
	return sum;
}

static void
trace_fw_object(const char *name, struct drm_i915_gem_object *obj)
{
	uint8_t *p = i915_gem_object_pin_map_unlocked(obj, I915_MAP_WB);
	uint64_t size = obj->base.size;
	uint64_t off = 0;
	int region = 0;
	if ((uint64_t)(uintptr_t)p >= (uint64_t)-4095L) {
		trace_ptr("i915: map failed=", (uint64_t)(uintptr_t)p);
		return;
	}
	trace_ptr(name, size);
	while (off < size && region < 5) {
		uint32_t len = (uint32_t)(size - off);
		if (len > 65536)
			len = 65536;
		trace_ptr(region == 0 ? "i915: GEM r0=" :
		    region == 1 ? "i915: GEM r1=" :
		    region == 2 ? "i915: GEM r2=" :
		    region == 3 ? "i915: GEM r3=" : "i915: GEM r4=",
		    trace_sum(p + off, len));
		off += len;
		region++;
	}
	i915_gem_object_unpin_map(obj);
}

int __real_intel_guc_fw_upload(struct intel_guc *guc);

int __real_intel_guc_ct_send(struct intel_guc_ct *ct, const u32 *action,
    u32 len, u32 *response_buf, u32 response_buf_size, u32 flags);

int
__wrap_intel_guc_ct_send(struct intel_guc_ct *ct, const u32 *action, u32 len,
    u32 *response_buf, u32 response_buf_size, u32 flags)
{
	int ret;

	trace_ct = ct;
	trace_ptr("i915: CT action=", action[0]);
	if (action[0] == 0x550a) {
		trace_ptr("i915: CT 550a send head=", ct->ctbs.send.desc->head);
		trace_ptr("i915: CT 550a send tail=", ct->ctbs.send.desc->tail);
		trace_ptr("i915: CT 550a recv head=", ct->ctbs.recv.desc->head);
		trace_ptr("i915: CT 550a recv tail=", ct->ctbs.recv.desc->tail);
	}
	ret = __real_intel_guc_ct_send(ct, action, len, response_buf,
	    response_buf_size, flags);
	trace_ret("i915: CT ret", ret);
	if (action[0] == 0x550a) {
		trace_ret("i915: CT 550a ret", ret);
		trace_ptr("i915: CT 550a send head=", ct->ctbs.send.desc->head);
		trace_ptr("i915: CT 550a send tail=", ct->ctbs.send.desc->tail);
		trace_ptr("i915: CT 550a recv head=", ct->ctbs.recv.desc->head);
		trace_ptr("i915: CT 550a recv tail=", ct->ctbs.recv.desc->tail);
	}
	return ret;
}

void
i915_trace_ct_after_irq(uint64_t count)
{
	if (!trace_ct)
		return;
	trace_ptr("i915: MSI count=", count);
	trace_ptr("i915: CT send head=", trace_ct->ctbs.send.desc->head);
	trace_ptr("i915: CT send tail=", trace_ct->ctbs.send.desc->tail);
	trace_ptr("i915: CT recv head=", trace_ct->ctbs.recv.desc->head);
	trace_ptr("i915: CT recv tail=", trace_ct->ctbs.recv.desc->tail);
}

int
__wrap_intel_guc_fw_upload(struct intel_guc *guc)
{
	struct intel_uc_fw *fw = &guc->fw;
	struct intel_gt *gt = guc_to_gt(guc);
	bool present = false;
	bool local = false;
	dma_addr_t pte;
	uint8_t rsa[4096];
	size_t copied;

	trace_fw_object("i915: GuC GEM size=", fw->obj);
	copied = intel_uc_fw_copy_rsa(fw, rsa, sizeof(rsa));
	trace_ptr("i915: RSA source size=", copied);
	trace_ptr("i915: RSA source sum=", trace_sum(rsa, (uint32_t)copied));
	trace_ptr("i915: GuC GGTT offset=", fw->vma_res.start);
	pte = intel_ggtt_read_entry(&gt->ggtt->vm, fw->vma_res.start,
	    &present, &local);
	trace_ptr("i915: GuC GGTT pte=", pte);
	trace_ptr("i915: GuC GGTT flags=", (uint64_t)present | ((uint64_t)local << 1));
	if (fw->rsa_data) {
		trace_fw_object("i915: RSA GEM size=", fw->rsa_data->obj);
		trace_ptr("i915: RSA GGTT offset=", i915_ggtt_offset(fw->rsa_data));
		pte = intel_ggtt_read_entry(&gt->ggtt->vm,
		    i915_ggtt_offset(fw->rsa_data), &present, &local);
		trace_ptr("i915: RSA GGTT pte=", pte);
		trace_ptr("i915: RSA GGTT flags=", (uint64_t)present | ((uint64_t)local << 1));
	}
	return __real_intel_guc_fw_upload(guc);
}

unsigned long __real___px_dma(void *p);

static int px_trace_count = 0;

unsigned long
__wrap___px_dma(void *p)
{
	unsigned long pages = *(unsigned long *)((char *)p + 0x300);
	if (pages == 0) {
		if (px_trace_count < 8) {
			px_trace_count++;
			trace_ptr("px_dma NULL pages obj=", (uint64_t)(uintptr_t)p);
		}
		return 0;
	}
	unsigned long sgl = *(unsigned long *)pages;
	if (sgl == 0) {
		if (px_trace_count < 8) {
			px_trace_count++;
			trace_ptr("px_dma NULL sgl obj=", (uint64_t)(uintptr_t)p);
		}
		return 0;
	}
	return __real___px_dma(p);
}

void
__wrap_intel_pps_check_power_unlocked(void *intel_dp)
{
	(void)intel_dp;
}

int __real_intel_initial_commit(void *display);

int
__wrap_intel_initial_commit(void *display)
{
	os_print("i915: initial_commit ENTER");
	int r = __real_intel_initial_commit(display);
	trace_ptr("i915: initial_commit RET=", (uint64_t)(long)r);
	return r;
}

void *__real_i915_gem_object_create_region_at(void *mem, uint64_t offset,
    uint64_t size, unsigned int flags);

void *
__wrap_i915_gem_object_create_region_at(void *mem, uint64_t offset,
    uint64_t size, unsigned int flags)
{
	trace_ptr("i915: create_region_at offset=", offset);
	trace_ptr("i915:   size=", size);
	void *r = __real_i915_gem_object_create_region_at(mem, offset, size, flags);
	trace_ptr("i915:   create_region_at ret=", (uint64_t)(uintptr_t)r);
	return r;
}

void __real_intel_plane_disable_noatomic(void *crtc, void *plane);

void
__wrap_intel_plane_disable_noatomic(void *crtc, void *plane)
{
	os_print("i915: plane_disable_noatomic called (initial FB reconstruct failed)");
	__real_intel_plane_disable_noatomic(crtc, plane);
}

extern void *vm_page_to_virt(struct vm_page *pg);

void *
__wrap_vmap(void **pages, unsigned int npages, unsigned long flags,
    unsigned long prot)
{
	(void)flags;
	(void)prot;
	if (npages == 0 || pages == (void *)0 || pages[0] == (void *)0)
		return (void *)0;
	return vm_page_to_virt(pages[0]);
}

void *
__wrap_kmap(struct vm_page *pg)
{
	return vm_page_to_virt(pg);
}

void
__wrap_kunmap_va(void *addr)
{
	(void)addr;
}

void *
__wrap_kmap_atomic_prot(struct vm_page *pg, unsigned long prot)
{
	(void)prot;
	return vm_page_to_virt(pg);
}

void
__wrap_kunmap_atomic(void *addr)
{
	(void)addr;
}


void
__wrap_vga_get_uninterruptible(void *pdev, int rsrc)
{
	(void)pdev;
	(void)rsrc;
}

void
__wrap_vga_put(void *pdev, int rsrc)
{
	(void)pdev;
	(void)rsrc;
}

int __real_vlv_suspend_init(void *);
int __real_intel_region_ttm_device_init(void *);
int __real_intel_root_gt_init_early(void *);
int __real_intel_gt_probe_all(void *);

int
__wrap_vlv_suspend_init(void *a)
{
	int r;
	trace_enter("trace: -> vlv_suspend_init");
	r = __real_vlv_suspend_init(a);
	trace_ret("trace: <- vlv_suspend_init", r);
	return r;
}

int
__wrap_intel_region_ttm_device_init(void *a)
{
	int r;
	trace_enter("trace: -> intel_region_ttm_device_init");
	r = __real_intel_region_ttm_device_init(a);
	trace_ret("trace: <- intel_region_ttm_device_init", r);
	return r;
}

int
__wrap_intel_root_gt_init_early(void *a)
{
	int r;
	trace_enter("trace: -> intel_root_gt_init_early");
	r = __real_intel_root_gt_init_early(a);
	trace_ret("trace: <- intel_root_gt_init_early", r);
	return r;
}
int __real_intel_display_driver_probe_noirq(void *);
int __real_intel_irq_install(void *);
int __real_intel_display_driver_probe_nogem(void *);
int __real_i915_gem_init(void *);
int __real_intel_display_driver_probe(void *);

int
__wrap_intel_gt_probe_all(void *a)
{
	int r;
	trace_enter("trace: -> intel_gt_probe_all");
	r = __real_intel_gt_probe_all(a);
	trace_ret("trace: <- intel_gt_probe_all", r);
	return r;
}

int
__wrap_intel_display_driver_probe_noirq(void *a)
{
	int r;
	trace_enter("trace: -> intel_display_driver_probe_noirq");
	r = __real_intel_display_driver_probe_noirq(a);
	trace_ret("trace: <- intel_display_driver_probe_noirq", r);
	return r;
}

int
__wrap_intel_irq_install(void *a)
{
	int r;
	trace_enter("trace: -> intel_irq_install");
	r = __real_intel_irq_install(a);
	trace_ret("trace: <- intel_irq_install", r);
	return r;
}

int
__wrap_intel_display_driver_probe_nogem(void *a)
{
	int r;
	trace_enter("trace: -> intel_display_driver_probe_nogem");
	r = __real_intel_display_driver_probe_nogem(a);
	trace_ret("trace: <- intel_display_driver_probe_nogem", r);
	return r;
}

int
__wrap_i915_gem_init(void *a)
{
	int r;
	trace_enter("trace: -> i915_gem_init");
	r = __real_i915_gem_init(a);
	trace_ret("trace: <- i915_gem_init", r);
	return r;
}

int
__wrap_intel_display_driver_probe(void *a)
{
	int r;
	trace_enter("trace: -> intel_display_driver_probe");
	r = __real_intel_display_driver_probe(a);
	trace_ret("trace: <- intel_display_driver_probe", r);
	return r;
}

void *__real_i915_ppgtt_create(void *gt, unsigned long flags);
int __real_intel_engines_init(void *gt);
int __real_intel_gt_resume(void *gt);

void *
__wrap_i915_ppgtt_create(void *gt, unsigned long flags)
{
	trace_enter("trace: -> i915_ppgtt_create");
	void *r = __real_i915_ppgtt_create(gt, flags);
	trace_ptr("trace: <- i915_ppgtt_create=", (uint64_t)(uintptr_t)r);
	return r;
}

int
__wrap_intel_engines_init(void *gt)
{
	trace_enter("trace: -> intel_engines_init");
	int r = __real_intel_engines_init(gt);
	trace_ret("trace: <- intel_engines_init", r);
	return r;
}

int
__wrap_intel_gt_resume(void *gt)
{
	trace_enter("trace: -> intel_gt_resume");
	int r = __real_intel_gt_resume(gt);
	trace_ret("trace: <- intel_gt_resume", r);
	return r;
}

int __real_intel_execlists_submission_setup(void *engine);
void *__real_i915_gem_object_create_internal(void *i915, uint64_t size);
int __real_i915_vma_pin_ww(void *vma, void *ww, uint64_t size, uint64_t align,
    uint64_t flags);

int
__wrap_intel_execlists_submission_setup(void *engine)
{
	trace_enter("trace: -> execlists_submission_setup");
	int r = __real_intel_execlists_submission_setup(engine);
	trace_ret("trace: <- execlists_submission_setup", r);
	return r;
}

void *
__wrap_i915_gem_object_create_internal(void *i915, uint64_t size)
{
	void *r = __real_i915_gem_object_create_internal(i915, size);
	if ((uint64_t)(uintptr_t)r >= (uint64_t)-4095L) {
		os_print("\x1b[31m");
		trace_ptr("i915: create_internal FAILED size=", size);
		trace_ptr("i915:   err=", (uint64_t)(uintptr_t)r);
		os_print("\x1b[0m");
	}
	return r;
}

int
__wrap_i915_vma_pin_ww(void *vma, void *ww, uint64_t size, uint64_t align,
    uint64_t flags)
{
	int r = __real_i915_vma_pin_ww(vma, ww, size, align, flags);
	if (r < 0) {
		os_print("\x1b[31m");
		trace_ret("i915: vma_pin_ww FAILED", r);
		trace_ptr("i915:   flags=", flags);
		os_print("\x1b[0m");
	}
	return r;
}

int __real_i915_vma_get_pages(void *vma);
int __real_____i915_gem_object_get_pages(void *obj);

int
__wrap_i915_vma_get_pages(void *vma)
{
	int r = __real_i915_vma_get_pages(vma);
	if (r < 0) {
		os_print("\x1b[31m");
		trace_ret("VMA_GET_PAGES FAILED", r);
		os_print("\x1b[0m");
	}
	return r;
}

int
__wrap_____i915_gem_object_get_pages(void *obj)
{
	int r = __real_____i915_gem_object_get_pages(obj);
	if (r < 0) {
		os_print("\x1b[31m");
		trace_ptr("GET_PAGES FAILED obj=", (uint64_t)(uintptr_t)obj);
		trace_ret("  ret", r);
		os_print("\x1b[0m");
	}
	return r;
}

void __real_i915_request_add(void *rq);
void
__wrap_i915_request_add(void *rq)
{
	struct i915_request *request = rq;

	trace_ptr("trace: req_add rq=", (uint64_t)(uintptr_t)rq);
	trace_ptr("trace: req engine=", request->engine->id);
	trace_ptr("trace: req seqno=", request->fence.seqno);
	trace_ptr("trace: req hwsp=", *request->hwsp_seqno);
	__real_i915_request_add(rq);
	trace_enter("trace: req_add done\n");
}

int __real_intel_gt_wait_for_idle(void *gt, long timeout);
int
__wrap_intel_gt_wait_for_idle(void *gt, long timeout)
{
	trace_ptr("trace: -> gt_wait_for_idle timeout=", (uint64_t)timeout);
	int r = __real_intel_gt_wait_for_idle(gt, timeout);
	trace_ret("trace: <- gt_wait_for_idle", r);
	return r;
}

void __real_intel_renderstate_fini(void *so, void *ce);
void
__wrap_intel_renderstate_fini(void *so, void *ce)
{
	trace_enter("trace: -> rstate_fini\n");
	__real_intel_renderstate_fini(so, ce);
	trace_enter("trace: <- rstate_fini\n");
}

long __real_dma_fence_wait_timeout(void *fence, int intr, long timeout);
long
__wrap_dma_fence_wait_timeout(void *fence, int intr, long timeout)
{
	trace_ptr("trace: -> dma_fence_wait_timeout to=", (uint64_t)timeout);
	long r = __real_dma_fence_wait_timeout(fence, intr, timeout);
	trace_ret("trace: <- dma_fence_wait_timeout", (int)r);
	return r;
}
