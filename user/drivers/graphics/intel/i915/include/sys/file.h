#pragma once
#include <stdint.h>
#include <sys/types.h>
#include <sys/mutex.h>

struct file;
struct uio;
struct knote;
struct stat;
struct proc;
struct filedesc;

struct fileops {
	int (*fo_read)(struct file *, struct uio *, int);
	int (*fo_write)(struct file *, struct uio *, int);
	int (*fo_ioctl)(struct file *, unsigned long, char *, struct proc *);
	int (*fo_kqfilter)(struct file *, struct knote *);
	int (*fo_stat)(struct file *, struct stat *, struct proc *);
	int (*fo_close)(struct file *, struct proc *);
	int (*fo_seek)(struct file *, off_t *, int, struct proc *);
};

struct file {
	void *f_data;
	int f_type;
	const struct fileops *f_ops;
	struct mutex f_mtx;
	off_t f_offset;
	int f_count;
	int f_flag;
};

#define FNONBLOCK 0x0004

#define DTYPE_DMABUF 10
#define DTYPE_SYNC 11

#define O_CLOEXEC 0x400000
#define UF_EXCLOSE 0x01

#define KERNEL_LOCK() ((void)0)
#define KERNEL_UNLOCK() ((void)0)

static inline struct file *
fnew(struct proc *p)
{
	(void)p;
	return 0;
}

static inline struct file *
fd_getfile(struct filedesc *fdp, int fd)
{
	(void)fdp;
	(void)fd;
	return 0;
}

static inline void
FRELE(struct file *fp, struct proc *p)
{
	(void)fp;
	(void)p;
}

static inline void
FREF(struct file *fp)
{
	(void)fp;
}

static inline void
fdplock(struct filedesc *fdp)
{
	(void)fdp;
}

static inline void
fdpunlock(struct filedesc *fdp)
{
	(void)fdp;
}

static inline int
fdalloc(struct proc *p, int want, int *result)
{
	(void)p;
	(void)want;
	(void)result;
	return -1;
}

static inline void
fdexpand(struct proc *p)
{
	(void)p;
}

static inline void
fdinsert(struct filedesc *fdp, int fd, int flags, struct file *fp)
{
	(void)fdp;
	(void)fd;
	(void)flags;
	(void)fp;
}

static inline void
fdremove(struct filedesc *fdp, int fd)
{
	(void)fdp;
	(void)fd;
}

