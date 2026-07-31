#pragma once
#define NAGP 0

struct agp_info {
	unsigned long ai_aperture_base;
	unsigned long ai_aperture_size;
};

struct device;

struct agp_methods {
	void (*bind_page)(struct device *, unsigned long, unsigned long, int);
	void (*unbind_page)(struct device *, unsigned long);
};

struct agp_softc {
	struct device *sc_chipc;
	unsigned long sc_apaddr;
	const struct agp_methods *sc_methods;
};
