#pragma once

struct klist {
	int dummy;
};

#define NOTE_CHANGE 0x0001
#define NOTE_SUBMIT 0x0100
#define FILTEROP_ISFD 0x01
#define EVFILT_READ (-1)
#define EVFILT_DEVICE (-8)

struct knote;

struct filterops {
	int f_flags;
	void *f_attach;
	void (*f_detach)(struct knote *);
	int (*f_event)(struct knote *, long);
};

static inline void
knote_locked(struct klist *list, long hint)
{
	(void)list;
	(void)hint;
}

struct knote {
	void *kn_hook;
	long kn_sfflags;
	long kn_fflags;
	int kn_filter;
	const struct filterops *kn_fop;
};

static inline void
klist_remove_locked(struct klist *list, struct knote *kn)
{
	(void)list;
	(void)kn;
}

static inline void
klist_insert_locked(struct klist *list, struct knote *kn)
{
	(void)list;
	(void)kn;
}
