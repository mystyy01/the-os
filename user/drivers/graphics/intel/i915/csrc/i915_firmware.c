#include <stddef.h>
#include <stdint.h>
#include <errno.h>

extern void *compat_arena_alloc(size_t size);
extern long os_load_file(const char *path, unsigned char *out, uint64_t max);
extern void os_print(const char *s);
extern void *memset(void *s, int c, size_t n);

#define FW_MAX_SIZE (4u * 1024u * 1024u)

int
loadfirmware(const char *name, unsigned char **bufp, size_t *buflen)
{
	char path[256];
	static const char prefix[] = "/lib/firmware/";
	int i = 0;
	int j = 0;

	while (prefix[j] != 0 && i < (int)sizeof(path) - 1)
		path[i++] = prefix[j++];
	j = 0;
	while (name[j] != 0 && i < (int)sizeof(path) - 1)
		path[i++] = name[j++];
	path[i] = 0;

	unsigned char *buf = compat_arena_alloc(FW_MAX_SIZE);
	if (buf == NULL)
		return ENOMEM;
	memset(buf, 0, FW_MAX_SIZE);

	long n = os_load_file(path, buf, FW_MAX_SIZE);
	if (n <= 0) {
		os_print("i915: firmware load failed: ");
		os_print(path);
		os_print("\n");
		return ENOENT;
	}

	static const char hx[] = "0123456789abcdef";
	char msg[160];
	int p = 0;
	const char *lbl = "i915: fw len=";
	for (int q = 0; lbl[q]; q++) msg[p++] = lbl[q];
	for (int s = 28; s >= 0; s -= 4) msg[p++] = hx[((uint32_t)n >> s) & 0xf];
	long off = 0;
	int region = 0;
	while (off < n) {
		long end = off + 65536;
		if (end > n) end = n;
		uint32_t sum = 0;
		for (long k = off; k < end; k++)
			sum += buf[k];
		msg[p++] = ' ';
		msg[p++] = 'r';
		msg[p++] = hx[region & 0xf];
		msg[p++] = '=';
		for (int s = 28; s >= 0; s -= 4) msg[p++] = hx[(sum >> s) & 0xf];
		off = end;
		region++;
	}
	msg[p++] = '\n';
	msg[p] = 0;
	os_print(msg);

	*bufp = buf;
	*buflen = (size_t)n;
	return 0;
}
