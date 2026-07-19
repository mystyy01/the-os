#include <dev/pci/pcivar.h>

int
pci_enumerate_bus(struct pci_softc *sc, int (*match)(struct pci_attach_args *),
    struct pci_attach_args *pa)
{
	(void)sc;
	(void)match;
	(void)pa;
	return 0;
}
