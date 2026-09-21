#include <stdint.h>
#include <linux/rcupdate.h>

extern void i915_task_add(void *owner, void (*func)(void *), void *arg);
extern uint64_t os_sleep(uint64_t ident, uint64_t timeout_ns);

#define RCU_SLOTS 256
#define RCU_SLEEP_IDENT 0x1915524355424152ULL

struct rcu_slot {
	struct rcu_head *head;
	void (*func)(struct rcu_head *);
	int active;
};

static struct rcu_slot slots[RCU_SLOTS];
static unsigned int readers;
static unsigned int pending;
static int drain_queued;
static int lock_word;

static void
lock(void)
{
	while (__atomic_exchange_n(&lock_word, 1, __ATOMIC_ACQUIRE))
		__asm volatile("pause");
}

static void
unlock(void)
{
	__atomic_store_n(&lock_word, 0, __ATOMIC_RELEASE);
}

void
i915_rcu_read_lock(void)
{
	__atomic_add_fetch(&readers, 1, __ATOMIC_ACQUIRE);
}

void
i915_rcu_read_unlock(void)
{
	__atomic_sub_fetch(&readers, 1, __ATOMIC_RELEASE);
}

static void rcu_drain(void *unused);

static void
queue_drain(void)
{
	i915_task_add(&drain_queued, rcu_drain, 0);
}

void
i915_call_rcu(struct rcu_head *head, void (*func)(struct rcu_head *))
{
	int queue = 0;

	lock();
	for (int i = 0; i < RCU_SLOTS; i++) {
		if (!slots[i].active) {
			slots[i].head = head;
			slots[i].func = func;
			slots[i].active = 1;
			pending++;
			if (!drain_queued) {
				drain_queued = 1;
				queue = 1;
			}
			unlock();
			if (queue)
				queue_drain();
			return;
		}
	}
	unlock();

	for (;;)
		os_sleep(RCU_SLEEP_IDENT, 1000000);
}

static void
rcu_drain(void *unused)
{
	(void)unused;

	if (__atomic_load_n(&readers, __ATOMIC_ACQUIRE) != 0) {
		queue_drain();
		return;
	}

	for (;;) {
		struct rcu_head *head = 0;
		void (*func)(struct rcu_head *) = 0;

		lock();
		for (int i = 0; i < RCU_SLOTS; i++) {
			if (slots[i].active) {
				head = slots[i].head;
				func = slots[i].func;
				slots[i].active = 0;
				pending--;
				break;
			}
		}
		if (!head)
			drain_queued = 0;
		unlock();

		if (!head)
			return;
		func(head);

		if (__atomic_load_n(&readers, __ATOMIC_ACQUIRE) != 0) {
			queue_drain();
			return;
		}
	}
}

void
i915_rcu_barrier(void)
{
	while (__atomic_load_n(&pending, __ATOMIC_ACQUIRE) != 0)
		os_sleep(RCU_SLEEP_IDENT, 1000000);
}
