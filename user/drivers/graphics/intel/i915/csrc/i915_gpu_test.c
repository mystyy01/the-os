#include "i915_drv.h"
#include "i915_request.h"
#include "i915_vma.h"
#include <drm/drm_client.h>
#include <drm/drm_fourcc.h>
#include "display/intel_display_core.h"
#include "display/intel_display_types.h"
#include "display/intel_de.h"
#include "display/intel_fb.h"
#include "display/intel_fbdev.h"
#include "display/intel_frontbuffer.h"
#include "display/skl_universal_plane_regs.h"
#include "gem/i915_gem_internal.h"
#include "gt/intel_context.h"
#include "gt/intel_engine.h"
#include "gt/intel_gpu_commands.h"
#include "gt/intel_gt.h"

struct bounce_state {
	struct drm_i915_gem_object *fb_obj;
	struct drm_i915_gem_object *batch_obj;
	struct i915_vma *fb_vma;
	struct i915_vma *batch_vma;
	struct intel_engine_cs *engine;
	struct intel_framebuffer *fb;
	struct drm_i915_private *i915;
	struct i915_vma *scanout_vma;
	uint32_t width;
	uint32_t height;
	uint32_t pitch;
	int fb_active;
	int batch_active;
	int ready;
};

static struct bounce_state bounce;

void i915_shim_commit_complete(void);
int printf_direct(const char *fmt, ...);
void i915_platform_report(struct drm_i915_private *);
extern unsigned long __drm_debug;
int i915_shim_draw_square(uint32_t x, uint32_t y, uint32_t w, uint32_t h,
    uint32_t rgb);

static void
bounce_diag_mark(uint32_t color)
{
	if (bounce.width >= 96 && bounce.height >= 96)
		i915_shim_draw_square(bounce.width - 96, 32, 64, 64, color);
}

int
i915_shim_gpu_exec_test(struct intel_gt *gt)
{
	struct drm_i915_gem_object *result_obj;
	struct drm_i915_gem_object *batch_obj;
	struct i915_vma *result_vma;
	struct i915_vma *batch_vma;
	struct intel_engine_cs *engine = NULL;
	struct i915_request *rq;
	struct i915_gem_ww_ctx ww;
	enum intel_engine_id id;
	uint32_t *result;
	uint32_t *batch;
	uint64_t address;
	long wait;
	int result_pinned = 0;
	int batch_pinned = 0;
	int err;

	for_each_engine(engine, gt, id) {
		if (engine->class == RENDER_CLASS &&
		    intel_engine_can_store_dword(engine))
			break;
	}
	if (!engine || engine->class != RENDER_CLASS)
		return -ENODEV;

	result_obj = i915_gem_object_create_internal(gt->i915, PAGE_SIZE);
	if (IS_ERR(result_obj))
		return PTR_ERR(result_obj);

	batch_obj = i915_gem_object_create_internal(gt->i915, PAGE_SIZE);
	if (IS_ERR(batch_obj)) {
		err = PTR_ERR(batch_obj);
		goto put_result;
	}

	result_vma = i915_vma_instance(result_obj,
	    engine->kernel_context->vm, NULL);
	if (IS_ERR(result_vma)) {
		err = PTR_ERR(result_vma);
		goto put_batch;
	}

	batch_vma = i915_vma_instance(batch_obj,
	    engine->kernel_context->vm, NULL);
	if (IS_ERR(batch_vma)) {
		err = PTR_ERR(batch_vma);
		goto put_batch;
	}

	i915_gem_ww_ctx_init(&ww, false);
retry:
	err = i915_gem_object_lock(result_obj, &ww);
	if (!err)
		err = i915_gem_object_lock(batch_obj, &ww);
	if (!err) {
		err = i915_vma_pin_ww(result_vma, &ww, 0, 0, PIN_USER);
		if (!err)
			result_pinned = 1;
	}
	if (!err) {
		err = i915_vma_pin_ww(batch_vma, &ww, 0, 0, PIN_USER);
		if (!err)
			batch_pinned = 1;
	}
	if (err == -EDEADLK) {
		if (batch_pinned) {
			i915_vma_unpin(batch_vma);
			batch_pinned = 0;
		}
		if (result_pinned) {
			i915_vma_unpin(result_vma);
			result_pinned = 0;
		}
		err = i915_gem_ww_ctx_backoff(&ww);
		if (!err)
			goto retry;
	}
	if (err)
		goto fini_ww;

	result = i915_gem_object_pin_map(result_obj, I915_MAP_WB);
	if (IS_ERR(result)) {
		err = PTR_ERR(result);
		goto unpin_vmas;
	}
	*result = 0;
	i915_gem_object_flush_map(result_obj);
	i915_gem_object_unpin_map(result_obj);

	address = i915_vma_offset(result_vma);
	batch = i915_gem_object_pin_map(batch_obj, I915_MAP_WC);
	if (IS_ERR(batch)) {
		err = PTR_ERR(batch);
		goto unpin_vmas;
	}
	*batch++ = MI_STORE_DWORD_IMM_GEN4;
	*batch++ = lower_32_bits(address);
	*batch++ = upper_32_bits(address);
	*batch++ = 0xc0dec0de;
	*batch++ = MI_BATCH_BUFFER_END;
	i915_gem_object_flush_map(batch_obj);
	i915_gem_object_unpin_map(batch_obj);
	intel_gt_chipset_flush(gt);

	rq = i915_request_create(engine->kernel_context);
	if (IS_ERR(rq)) {
		err = PTR_ERR(rq);
		goto unpin_vmas;
	}

	err = i915_vma_move_to_active(batch_vma, rq, 0);
	if (!err)
		err = i915_vma_move_to_active(result_vma, rq,
		    EXEC_OBJECT_WRITE);
	if (!err)
		err = rq->engine->emit_bb_start(rq,
		    i915_vma_offset(batch_vma), PAGE_SIZE, 0);

	i915_request_get(rq);
	if (err)
		i915_request_set_error_once(rq, err);
	i915_request_add(rq);

	wait = i915_request_wait(rq, 0, HZ);
	if (wait <= 0) {
		if (!err)
			err = wait < 0 ? wait : -ETIME;
		printf("i915: GPU_EXEC timeout=%ld\n", wait);
		goto put_request;
	}
	if (err)
		goto put_request;

	result = i915_gem_object_pin_map(result_obj, I915_MAP_WB);
	if (IS_ERR(result)) {
		err = PTR_ERR(result);
		goto put_request;
	}
	if (*result == 0xc0dec0de) {
		printf("i915: GPU_EXEC PASS value=0x%x engine=%s\n",
		    *result, engine->name);
		err = 0;
	} else {
		printf("i915: GPU_EXEC FAIL value=0x%x engine=%s\n",
		    *result, engine->name);
		err = -EIO;
	}
	i915_gem_object_unpin_map(result_obj);

put_request:
	i915_request_put(rq);
unpin_vmas:
	if (batch_pinned)
		i915_vma_unpin(batch_vma);
	if (result_pinned)
		i915_vma_unpin(result_vma);
fini_ww:
	i915_gem_ww_ctx_fini(&ww);
put_batch:
	i915_gem_object_put(batch_obj);
put_result:
	i915_gem_object_put(result_obj);
	return err;
}

static uint32_t *
emit_fill(uint32_t *batch, uint64_t address, uint32_t pitch,
    uint32_t x, uint32_t y, uint32_t width, uint32_t height,
    uint32_t color)
{
	*batch++ = XY_COLOR_BLT_CMD | BLT_WRITE_RGBA | (7 - 2);
	*batch++ = BLT_DEPTH_32 | BLT_ROP_COLOR_COPY | pitch;
	*batch++ = y << 16 | x;
	*batch++ = (y + height) << 16 | (x + width);
	*batch++ = lower_32_bits(address);
	*batch++ = upper_32_bits(address);
	*batch++ = color;
	*batch++ = MI_NOOP;
	return batch;
}

static int
bounce_wait(struct i915_request *rq)
{
	long wait;

	wait = i915_request_wait(rq, 0, HZ);
	if (wait <= 0)
		return wait < 0 ? (int)wait : -ETIME;

	return 0;
}

static int
bounce_submit(uint32_t old_x, uint32_t old_y, uint32_t new_x,
    uint32_t new_y, uint32_t size, uint32_t color, int clear)
{
	static uint32_t frame;
	static int traced;
	struct i915_gem_ww_ctx ww;
	struct i915_request *rq;
	uint32_t *batch;
	uint64_t address;
	int trace = !clear && !traced;
	int err;
	int deadlock_retries = 0;
	int ww_active = 1;

	if (trace) {
		traced = 1;
		printf("i915: GPU_BOUNCE first submit enter\n");
		bounce_diag_mark(0x00ffff00);
	}
	i915_gem_ww_ctx_init(&ww, false);
retry:
	if (trace)
		printf("i915: GPU_BOUNCE locking framebuffer\n");
	err = i915_gem_object_lock(bounce.fb_obj, &ww);
	if (trace)
		printf("i915: GPU_BOUNCE framebuffer locked\n");
	if (!err)
		err = i915_gem_object_lock(bounce.batch_obj, &ww);
	if (trace)
		printf("i915: GPU_BOUNCE batch lock returned\n");
	if (err == -EDEADLK) {
		if (++deadlock_retries > 1000) {
			err = -EDEADLK;
			goto fini_ww;
		}
		err = i915_gem_ww_ctx_backoff(&ww);
		if (!err)
			goto retry;
	}
	if (err)
		goto fini_ww;
	if (trace)
		bounce_diag_mark(0x0000ffff);

	address = i915_vma_offset(bounce.fb_vma) +
	    bounce.fb->base.offsets[0];
	batch = i915_gem_object_pin_map(bounce.batch_obj, I915_MAP_WC);
	if (IS_ERR(batch)) {
		err = PTR_ERR(batch);
		goto fini_ww;
	}
	if (trace)
		printf("i915: GPU_BOUNCE batch mapped\n");

	if (clear) {
		batch = emit_fill(batch, address, bounce.pitch, 0, 0,
		    bounce.width, bounce.height, 0x00101820);
	} else {
		batch = emit_fill(batch, address, bounce.pitch, old_x, old_y,
		    size, size, 0x00101820);
	}
	batch = emit_fill(batch, address, bounce.pitch, new_x, new_y,
	    size, size, color);
	*batch++ = MI_BATCH_BUFFER_END;
	*batch = MI_NOOP;
	i915_gem_object_flush_map(bounce.batch_obj);
	i915_gem_object_unpin_map(bounce.batch_obj);
	intel_gt_chipset_flush(bounce.engine->gt);

	rq = i915_request_create(bounce.engine->kernel_context);
	if (IS_ERR(rq)) {
		err = PTR_ERR(rq);
		goto fini_ww;
	}
	if (trace)
		bounce_diag_mark(0x000000ff);
	if (trace)
		printf("i915: GPU_BOUNCE request created\n");

	if (trace)
		printf("i915: GPU_BOUNCE batch await enter\n");
	err = i915_request_await_object(rq, bounce.batch_obj, false);
	if (trace)
		printf("i915: GPU_BOUNCE batch await returned\n");
	if (!err)
		err = _i915_vma_move_to_active(bounce.batch_vma, rq,
		    NULL, __EXEC_OBJECT_NO_REQUEST_AWAIT);
	if (trace)
		printf("i915: GPU_BOUNCE batch active returned\n");
	if (!err)
		err = _i915_vma_move_to_active(bounce.fb_vma, rq, NULL,
		    EXEC_OBJECT_WRITE | __EXEC_OBJECT_NO_REQUEST_AWAIT);
	if (trace)
		printf("i915: GPU_BOUNCE framebuffer active returned\n");
	if (!err)
		err = rq->engine->emit_bb_start(rq,
		    i915_vma_offset(bounce.batch_vma), PAGE_SIZE, 0);
	if (trace)
		printf("i915: GPU_BOUNCE batch start returned\n");

	i915_request_get(rq);
	if (err)
		i915_request_set_error_once(rq, err);
	if (trace)
		printf("i915: GPU_BOUNCE frontbuffer invalidate enter\n");
	intel_frontbuffer_invalidate(bounce.fb->frontbuffer, ORIGIN_CS);
	if (trace)
		printf("i915: GPU_BOUNCE frontbuffer invalidate returned\n");
	if (trace)
		printf("i915: GPU_BOUNCE request adding\n");
	i915_request_add(rq);
	if (trace)
		bounce_diag_mark(0x00ff00ff);

	i915_gem_ww_ctx_fini(&ww);
	ww_active = 0;

	if (!err)
		err = bounce_wait(rq);
	if (trace && !err)
		bounce_diag_mark(0x0000ff00);
	if (trace)
		printf("i915: GPU_BOUNCE request wait returned\n");
	if (!clear && !err) {
		frame++;
		if (frame % 120 == 0) {
			printf("i915: GPU_BOUNCE heartbeat frame=%u pos=%u,%u "
			    "seq=%u hwsp=%u surf=%08x live=%08x ggtt=%llx\n",
			    frame, new_x, new_y, rq->fence.seqno,
			    hwsp_seqno(rq),
			    intel_de_read(bounce.i915->display,
				PLANE_SURF(PIPE_A, PLANE_PRIMARY)),
			    intel_de_read(bounce.i915->display,
				PLANE_SURFLIVE(PIPE_A, PLANE_PRIMARY)),
			    (unsigned long long)i915_vma_offset(
				bounce.scanout_vma));
		}
	}
	i915_request_put(rq);
	intel_frontbuffer_flush(bounce.fb->frontbuffer, ORIGIN_CS);

fini_ww:
	if (err) {
		if (trace)
			bounce_diag_mark(0x00ff0000);
		printf_direct("i915: GPU_BOUNCE frame=%u err=%d retries=%d\n",
		    frame, err, deadlock_retries);
	}
	if (ww_active)
		i915_gem_ww_ctx_fini(&ww);
	return err;
}

int
i915_shim_gpu_bounce_setup(struct drm_i915_private *i915)
{
	struct intel_fbdev *fbdev = i915->display->fbdev.fbdev;
	struct intel_engine_cs *engine = NULL;
	struct i915_gem_ww_ctx ww;
	enum intel_engine_id id;
	int fb_pinned = 0;
	int batch_pinned = 0;
	int err;

	if (!fbdev)
		return -ENODEV;

	bounce.fb = intel_fbdev_framebuffer(fbdev);
	if (!bounce.fb ||
	    bounce.fb->base.format->format != DRM_FORMAT_XRGB8888 ||
	    bounce.fb->base.modifier != DRM_FORMAT_MOD_LINEAR)
		return -EINVAL;

	for_each_engine(engine, i915->gt[0], id) {
		if (engine->class == COPY_ENGINE_CLASS)
			break;
	}
	if (!engine || engine->class != COPY_ENGINE_CLASS)
		return -ENODEV;

	bounce.engine = engine;
	bounce.i915 = i915;
	bounce.scanout_vma = intel_fbdev_vma_pointer(fbdev);
	bounce.width = bounce.fb->base.width;
	bounce.height = bounce.fb->base.height;
	bounce.pitch = bounce.fb->base.pitches[0];
	bounce.fb_obj = i915_gem_object_get(
	    to_intel_bo(intel_fb_bo(&bounce.fb->base)));
	bounce.batch_obj = i915_gem_object_create_internal(i915, PAGE_SIZE);
	if (IS_ERR(bounce.batch_obj)) {
		err = PTR_ERR(bounce.batch_obj);
		goto put_fb;
	}

	bounce.fb_vma = i915_vma_instance(bounce.fb_obj,
	    engine->kernel_context->vm, NULL);
	if (IS_ERR(bounce.fb_vma)) {
		err = PTR_ERR(bounce.fb_vma);
		goto put_batch;
	}
	bounce.batch_vma = i915_vma_instance(bounce.batch_obj,
	    engine->kernel_context->vm, NULL);
	if (IS_ERR(bounce.batch_vma)) {
		err = PTR_ERR(bounce.batch_vma);
		goto put_batch;
	}

	i915_gem_ww_ctx_init(&ww, false);
retry:
	err = i915_gem_object_lock(bounce.fb_obj, &ww);
	if (!err)
		err = i915_gem_object_lock(bounce.batch_obj, &ww);
	if (!err) {
		err = i915_vma_pin_ww(bounce.fb_vma, &ww, 0, 0, PIN_USER);
		if (!err)
			fb_pinned = 1;
	}
	if (!err) {
		err = i915_vma_pin_ww(bounce.batch_vma, &ww, 0, 0, PIN_USER);
		if (!err)
			batch_pinned = 1;
	}
	if (err == -EDEADLK) {
		if (batch_pinned) {
			i915_vma_unpin(bounce.batch_vma);
			batch_pinned = 0;
		}
		if (fb_pinned) {
			i915_vma_unpin(bounce.fb_vma);
			fb_pinned = 0;
		}
		err = i915_gem_ww_ctx_backoff(&ww);
		if (!err)
			goto retry;
	}
	i915_gem_ww_ctx_fini(&ww);
	if (err)
		goto unpin_failed;

	err = i915_active_acquire(&bounce.batch_vma->active);
	if (err)
		goto unpin;
	bounce.batch_active = 1;

	err = i915_active_acquire(&bounce.fb_vma->active);
	if (err)
		goto release_batch_active;
	bounce.fb_active = 1;

	err = bounce_submit(0, 0, 32, 32, 128, 0x0000d8ff, 1);
	if (err)
		goto release_fb_active;

	bounce.ready = 1;
	printf("i915: GPU_BOUNCE ready engine=%s %ux%u pitch=%u\n",
	    engine->name, bounce.width, bounce.height, bounce.pitch);
	return 0;

release_fb_active:
	i915_active_release(&bounce.fb_vma->active);
	bounce.fb_active = 0;
release_batch_active:
	i915_active_release(&bounce.batch_vma->active);
	bounce.batch_active = 0;
unpin:
	i915_vma_unpin(bounce.batch_vma);
	i915_vma_unpin(bounce.fb_vma);
	goto put_batch;
unpin_failed:
	if (batch_pinned)
		i915_vma_unpin(bounce.batch_vma);
	if (fb_pinned)
		i915_vma_unpin(bounce.fb_vma);
put_batch:
	i915_gem_object_put(bounce.batch_obj);
put_fb:
	i915_gem_object_put(bounce.fb_obj);
	return err;
}

void
i915_shim_gpu_bounce_start(struct drm_i915_private *i915)
{
	int err;

	if (bounce.ready)
		return;

	i915_platform_report(i915);
	printf_direct("i915: GPU_BOUNCE setup start\n");
	err = i915_shim_gpu_bounce_setup(i915);
	if (err) {
		printf_direct("i915: GPU_BOUNCE setup failed ret=%d\n", err);
		return;
	}

	printf_direct("i915: GPU_BOUNCE setup done\n");
	__drm_debug = 0;
	i915_shim_commit_complete();
}

void
i915_shim_fbdev_commit_complete(struct drm_device *dev)
{
	i915_shim_gpu_bounce_start(to_i915(dev));
}

int
i915_shim_gpu_bounce_frame(uint32_t old_x, uint32_t old_y,
    uint32_t new_x, uint32_t new_y, uint32_t size, uint32_t color)
{
	if (!bounce.ready)
		return -ENODEV;
	if (!size || old_x + size > bounce.width ||
	    new_x + size > bounce.width || old_y + size > bounce.height ||
	    new_y + size > bounce.height)
		return -EINVAL;

	return bounce_submit(old_x, old_y, new_x, new_y, size, color, 0);
}
