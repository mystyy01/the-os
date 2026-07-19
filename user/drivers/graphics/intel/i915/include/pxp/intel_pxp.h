#pragma once
#include <stdbool.h>

struct drm_gem_object;
struct drm_i915_private;
struct intel_pxp;

static inline int
intel_pxp_key_check(struct drm_gem_object *obj, bool assign)
{
	(void)obj;
	(void)assign;
	return 0;
}

static inline int
intel_pxp_init(struct drm_i915_private *i915)
{
	(void)i915;
	return 0;
}

static inline void
intel_pxp_fini(struct drm_i915_private *i915)
{
	(void)i915;
}

static inline bool
intel_pxp_is_enabled(const struct intel_pxp *pxp)
{
	(void)pxp;
	return false;
}

static inline bool
intel_pxp_is_active(const struct intel_pxp *pxp)
{
	(void)pxp;
	return false;
}

static inline int
intel_pxp_start(struct intel_pxp *pxp)
{
	(void)pxp;
	return 0;
}

static inline int
intel_pxp_get_readiness_status(struct intel_pxp *pxp, int timeout_ms)
{
	(void)pxp;
	(void)timeout_ms;
	return 0;
}
