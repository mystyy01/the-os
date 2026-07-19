#pragma once
#include <stddef.h>

struct uio {
	size_t uio_resid;
};

static inline int
uiomove(void *buf, size_t n, struct uio *uio)
{
	(void)buf;
	(void)n;
	(void)uio;
	return 0;
}
