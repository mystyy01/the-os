#pragma once

#define IOCPARM_MASK 0x1fff
#define IOC_VOID 0x20000000
#define IOC_OUT 0x40000000
#define IOC_IN 0x80000000
#define IOC_INOUT (IOC_IN | IOC_OUT)

#define _IOC(inout, group, num, len) \
	((inout) | (((len) & IOCPARM_MASK) << 16) | ((group) << 8) | (num))

#define _IO(g, n) _IOC(IOC_VOID, (g), (n), 0)
#define _IOR(g, n, t) _IOC(IOC_OUT, (g), (n), sizeof(t))
#define _IOW(g, n, t) _IOC(IOC_IN, (g), (n), sizeof(t))
#define _IOWR(g, n, t) _IOC(IOC_INOUT, (g), (n), sizeof(t))
#define IOCPARM_LEN(x) (((x) >> 16) & IOCPARM_MASK)
