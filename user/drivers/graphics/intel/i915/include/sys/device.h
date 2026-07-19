#pragma once
#include <stddef.h>
#include <sys/types.h>

struct cfdriver {
	void **cd_devs;
	char *cd_name;
	int cd_class;
	int cd_ndevs;
};

struct cfattach {
	size_t ca_devsize;
	int (*ca_match)(struct device *, void *, void *);
	void (*ca_attach)(struct device *, struct device *, void *);
	int (*ca_detach)(struct device *, int);
	int (*ca_activate)(struct device *, int);
};

struct cfdata {
	struct cfdriver *cf_driver;
	struct cfattach *cf_attach;
	int *cf_loc;
};

#define DV_DULL 0

#define DVACT_QUIESCE 1
#define DVACT_SUSPEND 2
#define DVACT_RESUME 3
#define DVACT_WAKEUP 4

#define UNCONF 0

static inline int
config_suspend(struct device *dev, int act)
{
	(void)dev;
	(void)act;
	return 0;
}

static inline void
config_mountroot(struct device *dev, void (*hook)(struct device *))
{
	if (hook)
		hook(dev);
}

struct cdevsw {
	paddr_t (*d_mmap)(dev_t, off_t, int);
};

extern struct cdevsw cdevsw[];
paddr_t drmmmap(dev_t kdev, off_t offset, int prot);

static inline void *
config_found_sm(struct device *self, void *aux,
    int (*print)(void *, const char *),
    int (*submatch)(struct device *, void *, void *))
{
	(void)self;
	(void)aux;
	(void)print;
	(void)submatch;
	return NULL;
}
