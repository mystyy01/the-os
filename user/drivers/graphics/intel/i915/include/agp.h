#pragma once
#define NAGP 0

struct agp_info {
	unsigned long ai_aperture_base;
	unsigned long ai_aperture_size;
};

struct device;

struct agp_softc {
	struct device *sc_chipc;
};
