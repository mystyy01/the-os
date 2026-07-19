#pragma once

struct intel_pxp;

static inline void
intel_pxp_irq_handler(struct intel_pxp *pxp, unsigned short iir)
{
	(void)pxp;
	(void)iir;
}
