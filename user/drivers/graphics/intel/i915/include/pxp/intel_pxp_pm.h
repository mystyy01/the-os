#pragma once

struct intel_pxp;

static inline void
intel_pxp_resume_complete(struct intel_pxp *pxp)
{
	(void)pxp;
}

static inline void
intel_pxp_suspend_prepare(struct intel_pxp *pxp)
{
	(void)pxp;
}

static inline void
intel_pxp_suspend(struct intel_pxp *pxp)
{
	(void)pxp;
}

static inline void
intel_pxp_runtime_suspend(struct intel_pxp *pxp)
{
	(void)pxp;
}

static inline void
intel_pxp_runtime_resume(struct intel_pxp *pxp)
{
	(void)pxp;
}
