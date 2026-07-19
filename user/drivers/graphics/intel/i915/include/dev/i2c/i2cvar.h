#pragma once
#include <stdint.h>
#include <stddef.h>

typedef uint8_t u_int8_t;
typedef uint16_t i2c_addr_t;

struct i2c_controller {
	void *ic_cookie;
	int (*ic_acquire_bus)(void *, int);
	void (*ic_release_bus)(void *, int);
	int (*ic_send_start)(void *, int);
	int (*ic_send_stop)(void *, int);
	int (*ic_initiate_xfer)(void *, i2c_addr_t, int);
	int (*ic_read_byte)(void *, u_int8_t *, int);
	int (*ic_write_byte)(void *, u_int8_t, int);
};

typedef int i2c_op_t;
#define I2C_OP_READ 0
#define I2C_OP_WRITE 1
#define I2C_OP_READ_WITH_STOP 2
#define I2C_OP_WRITE_WITH_STOP 3

static inline int
iic_acquire_bus(struct i2c_controller *ic, int flags)
{
	(void)ic;
	(void)flags;
	return 0;
}

static inline int
iic_release_bus(struct i2c_controller *ic, int flags)
{
	(void)ic;
	(void)flags;
	return 0;
}

static inline int
iic_exec(struct i2c_controller *ic, i2c_op_t op, i2c_addr_t addr,
    const void *cmd, size_t cmdlen, void *buf, size_t buflen, int flags)
{
	(void)ic;
	(void)op;
	(void)addr;
	(void)cmd;
	(void)cmdlen;
	(void)buf;
	(void)buflen;
	(void)flags;
	return -1;
}
