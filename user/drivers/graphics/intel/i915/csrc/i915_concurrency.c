#include <stdint.h>
#include <stddef.h>


extern uint64_t os_sleep(uint64_t ident, uint64_t timeout_ns);
extern void os_wakeup(uint64_t ident);
extern uint64_t os_now_ns(void);
extern void i915_refresh_jiffies(void);
extern uint64_t i915_irq_dispatch(void);
extern void i915_trace_ct_after_irq(uint64_t count);

#define I915_WORKER_IDENT 0x1915000000000001ULL
#define I915_TICK_NS 10000000ULL

#define MAX_TIMEOUTS 64
#define MAX_TASKS 64

struct pending_timeout {
	void (*func)(void *);
	void *arg;
	void *owner;
	uint64_t deadline_ns;
	int active;
};

struct pending_task {
	void (*func)(void *);
	void *arg;
	void *owner;
	int active;
};

static struct pending_timeout g_timeouts[MAX_TIMEOUTS];
static struct pending_task g_tasks[MAX_TASKS];
void
i915_timeout_add_ns(void *owner, void (*func)(void *), void *arg, uint64_t delay_ns)
{
	uint64_t deadline = os_now_ns() + delay_ns;
	for (int i = 0; i < MAX_TIMEOUTS; i++) {
		if (!g_timeouts[i].active) {
			g_timeouts[i].func = func;
			g_timeouts[i].arg = arg;
			g_timeouts[i].owner = owner;
			g_timeouts[i].deadline_ns = deadline;
			g_timeouts[i].active = 1;
			os_wakeup(I915_WORKER_IDENT);
			return;
		}
	}
}

int
i915_timeout_del(void *owner)
{
	int found = 0;
	for (int i = 0; i < MAX_TIMEOUTS; i++) {
		if (g_timeouts[i].active && g_timeouts[i].owner == owner) {
			g_timeouts[i].active = 0;
			found = 1;
		}
	}
	return found;
}

void
i915_task_add(void *owner, void (*func)(void *), void *arg)
{
	for (int i = 0; i < MAX_TASKS; i++) {
		if (!g_tasks[i].active) {
			g_tasks[i].func = func;
			g_tasks[i].arg = arg;
			g_tasks[i].owner = owner;
			g_tasks[i].active = 1;
			os_wakeup(I915_WORKER_IDENT);
			return;
		}
	}
}

int
i915_task_del(void *owner)
{
	int found = 0;
	for (int i = 0; i < MAX_TASKS; i++) {
		if (g_tasks[i].active && g_tasks[i].owner == owner) {
			g_tasks[i].active = 0;
			found = 1;
		}
	}
	return found;
}

void
i915_worker_thread_entry(void)
{
	for (;;) {
		int did_work;

		i915_refresh_jiffies();
		{
			uint64_t irq_count = i915_irq_dispatch();
			if (irq_count)
				i915_trace_ct_after_irq(irq_count);
		}

		do {
			did_work = 0;
			uint64_t now = os_now_ns();

			for (int i = 0; i < MAX_TIMEOUTS; i++) {
				if (g_timeouts[i].active && now >= g_timeouts[i].deadline_ns) {
					void (*f)(void *) = g_timeouts[i].func;
					void *a = g_timeouts[i].arg;
					g_timeouts[i].active = 0;
					f(a);
					did_work = 1;
				}
			}
			for (int i = 0; i < MAX_TASKS; i++) {
				if (g_tasks[i].active) {
					void (*f)(void *) = g_tasks[i].func;
					void *a = g_tasks[i].arg;
					g_tasks[i].active = 0;
					f(a);
					did_work = 1;
				}
			}
		} while (did_work);

		uint64_t next_deadline = 0;
		int have_deadline = 0;
		for (int i = 0; i < MAX_TIMEOUTS; i++) {
			if (g_timeouts[i].active &&
			    (!have_deadline || g_timeouts[i].deadline_ns < next_deadline)) {
				next_deadline = g_timeouts[i].deadline_ns;
				have_deadline = 1;
			}
		}

		if (have_deadline) {
			uint64_t now = os_now_ns();
			uint64_t rel = (next_deadline > now) ? (next_deadline - now) : 1;
			if (rel > I915_TICK_NS)
				rel = I915_TICK_NS;
			os_sleep(I915_WORKER_IDENT, rel);
		} else {
			os_sleep(I915_WORKER_IDENT, I915_TICK_NS);
		}
	}
}
