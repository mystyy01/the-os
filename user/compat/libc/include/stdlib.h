#pragma once
#include <stddef.h>

void *malloc(size_t size);
void free(void *ptr);
void abort(void);
void *calloc(size_t n, size_t size);
void qsort(void *base, size_t n, size_t size,
	   int (*cmp)(const void *, const void *));
