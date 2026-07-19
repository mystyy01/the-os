#pragma once
#include <sys/types.h>

typedef unsigned int mode_t;

#define S_IFMT 0170000
#define S_IFREG 0100000
#define S_IFDIR 0040000
#define S_IFCHR 0020000
#define S_IFIFO 0010000
#define S_IRWXU 0000700
#define S_IRUSR 0000400
#define S_IWUSR 0000200

struct stat {
	off_t st_size;
	mode_t st_mode;
};
