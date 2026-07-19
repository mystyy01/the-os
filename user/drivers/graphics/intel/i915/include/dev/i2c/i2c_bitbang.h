#pragma once
#include <stdint.h>

#define I2C_BIT_SDA 0
#define I2C_BIT_SCL 1

struct i2c_bitbang_bits {
	uint32_t bit_sda;
	uint32_t bit_scl;
	uint32_t bit_output;
	uint32_t bit_input;
};

struct i2c_bitbang_ops {
	void (*ibo_set_bits)(void *, uint32_t);
	void (*ibo_set_dir)(void *, uint32_t);
	uint32_t (*ibo_read_bits)(void *);
	struct i2c_bitbang_bits ibo_bits;
};

static inline int
i2c_bitbang_send_start(void *cookie, int flags, const struct i2c_bitbang_ops *ops)
{
	(void)cookie;
	(void)flags;
	(void)ops;
	return 0;
}

static inline int
i2c_bitbang_send_stop(void *cookie, int flags, const struct i2c_bitbang_ops *ops)
{
	(void)cookie;
	(void)flags;
	(void)ops;
	return 0;
}

static inline int
i2c_bitbang_initiate_xfer(void *cookie, uint16_t addr, int flags,
    const struct i2c_bitbang_ops *ops)
{
	(void)cookie;
	(void)addr;
	(void)flags;
	(void)ops;
	return 0;
}

static inline int
i2c_bitbang_read_byte(void *cookie, uint8_t *bytep, int flags,
    const struct i2c_bitbang_ops *ops)
{
	(void)cookie;
	(void)flags;
	(void)ops;
	*bytep = 0;
	return -1;
}

static inline int
i2c_bitbang_write_byte(void *cookie, uint8_t byte, int flags,
    const struct i2c_bitbang_ops *ops)
{
	(void)cookie;
	(void)byte;
	(void)flags;
	(void)ops;
	return -1;
}
