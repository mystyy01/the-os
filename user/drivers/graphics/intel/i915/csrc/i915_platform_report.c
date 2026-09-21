#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <dev/pci/pcivar.h>
#include <dev/pci/pcireg.h>
#include <drm/drm_connector.h>
#include <drm/drm_device.h>
#include <drm/drm_mode_object.h>
#include <drm/drm_modes.h>

#include "i915_drv.h"
#include "intel_device_info.h"
#include "display/intel_display_core.h"
#include "display/intel_display_device.h"
#include "display/intel_pch.h"

int vsnprintf(char *, size_t, const char *, va_list);
void os_console_print(const char *);

static const char *
pch_name(enum intel_pch pch)
{
	switch (pch) {
	case PCH_NONE: return "NONE";
	case PCH_NOP: return "NOP";
	case PCH_SPT: return "SPT";
	case PCH_CNP: return "CNP";
	case PCH_ICP: return "ICP";
	case PCH_TGP: return "TGP";
	case PCH_ADP: return "ADP";
	default: return "other";
	}
}

static void
emit(const char *fmt, ...)
{
	char buf[192];
	va_list ap;

	va_start(ap, fmt);
	vsnprintf(buf, sizeof(buf), fmt, ap);
	va_end(ap);
	os_console_print(buf);
}

void
i915_platform_report(struct drm_i915_private *i915)
{
	struct intel_display *display = i915->display;
	struct drm_connector_list_iter iter;
	struct drm_connector *connector;
	pcitag_t tag;
	uint32_t id, rev, bar0, bar1, bar2, bar3;
	int nconn = 0;
	static int reported;

	if (reported)
		return;
	reported = 1;

	emit("PLATREPORT: platform=%s graphics_ver=%d devid=0x%04x\n",
	    intel_platform_name(INTEL_INFO(i915)->platform),
	    GRAPHICS_VER(i915), INTEL_DEVID(i915));

	tag = pci_make_tag(0, 0, 2, 0);
	id = pci_conf_read(0, tag, PCI_ID_REG);
	rev = pci_conf_read(0, tag, PCI_CLASS_REG) & 0xff;
	bar0 = pci_conf_read(0, tag, 0x10);
	bar1 = pci_conf_read(0, tag, 0x14);
	bar2 = pci_conf_read(0, tag, 0x18);
	bar3 = pci_conf_read(0, tag, 0x1c);
	emit("PLATREPORT: pci id=0x%08x rev=0x%02x\n", id, rev);
	emit("PLATREPORT: bar0=0x%08x bar1=0x%08x bar2=0x%08x bar3=0x%08x\n",
	    bar0, bar1, bar2, bar3);

	emit("PLATREPORT: display_ver=%d pch=%s(%d)\n",
	    DISPLAY_VER(display), pch_name(display->pch_type),
	    (int)display->pch_type);

	tag = pci_make_tag(0, 0, 31, 0);
	id = pci_conf_read(0, tag, PCI_ID_REG);
	emit("PLATREPORT: lpc id=0x%08x masked=0x%04x\n", id,
	    (uint32_t)(PCI_PRODUCT(id) & 0xff80));

	drm_connector_list_iter_begin(display->drm, &iter);
	drm_for_each_connector_iter(connector, &iter) {
		struct drm_display_mode *mode;
		int nmodes = 0;

		list_for_each_entry(mode, &connector->modes, head)
			nmodes++;

		emit("PLATREPORT: connector[%d] %s status=%s modes=%d edid=%s\n",
		    nconn, connector->name,
		    drm_get_connector_status_name(connector->status),
		    nmodes,
		    connector->edid_blob_ptr != NULL ? "yes" : "no");

		list_for_each_entry(mode, &connector->modes, head) {
			emit("PLATREPORT:   mode %dx%d@%d\n",
			    mode->hdisplay, mode->vdisplay, drm_mode_vrefresh(mode));
		}

		nconn++;
	}
	drm_connector_list_iter_end(&iter);

	emit("PLATREPORT: connectors=%d\n", nconn);
	os_console_print("PLATREPORT: end\n");
}
