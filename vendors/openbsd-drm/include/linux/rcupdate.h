/* Public domain. */

#ifndef LINUX_RCUPDATE_H
#define LINUX_RCUPDATE_H

#include <linux/cpumask.h>

struct rcu_head;
void i915_rcu_read_lock(void);
void i915_rcu_read_unlock(void);
void i915_call_rcu(struct rcu_head *, void (*)(struct rcu_head *));
void i915_rcu_barrier(void);

struct rcu_head {
};

#define __rcu
#define rcu_dereference(p)	(p)
#define rcu_dereference_raw(p)	(p)
#define rcu_dereference_protected(p, c)	(p)
#define rcu_dereference_check(p, c)	(p)
#define rcu_access_pointer(p)	(p)
#define RCU_INIT_POINTER(p, v)		do { (p) = (v); } while(0)
#define rcu_assign_pointer(p, v)	do { (p) = (v); } while(0)
#define rcu_read_lock()		i915_rcu_read_lock()
#define rcu_read_unlock()		i915_rcu_read_unlock()
#define rcu_pointer_handoff(p)	(p)
#define init_rcu_head(h)
#define destroy_rcu_head(h)

#define rcu_replace_pointer(rp, p, c)		\
({						\
	__typeof(rp) __r = rp;			\
	rp = p;					\
	__r;					\
})

#define kfree_rcu(objp, name)	do { free((void *)objp, M_DRM, 0); } while(0)

#define rcu_barrier()		i915_rcu_barrier()

typedef void (*rcu_callback_t)(struct rcu_head *head);

static inline void
call_rcu(struct rcu_head *head, void (*fn)(struct rcu_head *))
{
	i915_call_rcu(head, fn);
}

#define synchronize_rcu()
#define synchronize_rcu_expedited()
#define cond_synchronize_rcu(x)
#define get_state_synchronize_rcu()	0

#endif
