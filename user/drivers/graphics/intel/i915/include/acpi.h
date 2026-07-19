#pragma once
#define NACPI 0

#define ACPI_STATE_S4 4

struct acpi_softc {
	int sc_state;
};

extern struct acpi_softc *acpi_softc;
