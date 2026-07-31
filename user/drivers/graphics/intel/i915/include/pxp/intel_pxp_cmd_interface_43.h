#pragma once
#include <stdint.h>

#define PXP43_HUC_AUTH_INOUT_SIZE 4096

#define PXP_APIVER(major, minor) (((major) << 16) | (minor))
#define PXP43_CMDID_NEW_HUC_AUTH 0x0000003F

#define PXP_STATUS_SUCCESS 0x0
#define PXP_STATUS_OP_NOT_PERMITTED 0x1

struct pxp43_cmd_header {
	uint32_t api_version;
	uint32_t command_id;
	uint32_t status;
	uint32_t buffer_len;
};

struct pxp43_new_huc_auth_in {
	struct pxp43_cmd_header header;
	uint64_t huc_base_address;
	uint32_t huc_size;
};

struct pxp43_huc_auth_out {
	struct pxp43_cmd_header header;
};
