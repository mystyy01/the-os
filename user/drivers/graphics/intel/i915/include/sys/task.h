#pragma once

struct taskq {
	int dummy;
};

struct task {
	void (*t_func)(void *);
	void *t_arg;
	int t_pending;
};

#define TASK_INITIALIZER(func, arg) { func, arg, 0 }

static inline void
task_set(struct task *t, void (*func)(void *), void *arg)
{
	t->t_func = func;
	t->t_arg = arg;
	t->t_pending = 0;
}

static inline int
task_add(struct taskq *tq, struct task *t)
{
	(void)tq;
	t->t_pending = 0;
	t->t_func(t->t_arg);
	return 1;
}

static inline int
task_del(struct taskq *tq, struct task *t)
{
	(void)tq;
	(void)t;
	return 0;
}

#define task_pending(t) ((t)->t_pending)

static inline void
taskq_barrier(struct taskq *tq)
{
	(void)tq;
}

extern struct taskq *systq;

struct taskq *taskq_create(const char *name, int nthreads, int ipl, int flags);
void taskq_destroy(struct taskq *tq);
