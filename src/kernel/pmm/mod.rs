mod buddy;
mod spinlock;

use core::sync::atomic::AtomicBool;

pub use buddy::alloc_pages;
pub use buddy::free_pages;
pub use buddy::init;

pub const PAGE_SIZE: u64 = 4096;
pub const MAX_ORDER: usize = 11;

pub use spinlock::lock;
pub use spinlock::unlock;
