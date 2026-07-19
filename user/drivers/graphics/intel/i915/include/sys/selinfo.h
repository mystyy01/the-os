#pragma once
#include <sys/event.h>

struct selinfo {
	struct klist si_note;
};

static inline void
selwakeup(struct selinfo *sel)
{
	(void)sel;
}
