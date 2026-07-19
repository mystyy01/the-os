#pragma once
#include <sys/filedesc.h>
#include <sys/file.h>

struct process {
	int ps_pid;
	char ps_comm[24];
};

struct vmspace {
	struct vm_map vm_map;
};

struct proc {
	int p_pid;
	struct process *p_p;
	struct filedesc *p_fd;
	void *p_wchan;
	int p_stat;
	int p_flag;
	struct vmspace *p_vmspace;
};

extern struct proc *curproc;
int suser(struct proc *p);

#define SONPROC 7
#define P_INSCHED 0x0001
#define P_SINTR 0x0002
#define PPAUSE 0x28

static inline void
sleep_setup(void *wchan, int priority, const char *wmesg)
{
	(void)wchan;
	(void)priority;
	(void)wmesg;
}

static inline void
unsleep(struct proc *p)
{
	(void)p;
}

static inline int
wakeup_proc(struct proc *p)
{
	(void)p;
	return 0;
}

#define SCHED_LOCK() ((void)0)
#define SCHED_UNLOCK() ((void)0)
#define KERNEL_ASSERT_LOCKED() ((void)0)

extern char *hw_vendor;
extern char *hw_prod;
extern char *hw_ver;
