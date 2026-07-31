#include <sys/device.h>
#include <stddef.h>

int drmsubmatch(struct device *, void *, void *);
void drm_attach(struct device *, struct device *, void *);

struct drm_softc {
	struct device sc_dev;
	void *sc_drm;
	int sc_allocated;
};

static struct drm_softc g_drm_softc;

int
wsemuldisplaydevprint(void *aux, const char *pnp)
{
	(void)aux;
	(void)pnp;
	return 0;
}

int
wsemuldisplaydevsubmatch(struct device *parent, void *match, void *aux)
{
	(void)parent;
	(void)match;
	(void)aux;
	return 0;
}

void *
config_found_sm(struct device *self, void *aux,
    int (*print)(void *, const char *),
    int (*submatch)(struct device *, void *, void *))
{
	(void)print;

	if (submatch == drmsubmatch) {
		drm_attach(self, (struct device *)&g_drm_softc, aux);
		return &g_drm_softc;
	}

	return NULL;
}
