use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread_local;
use std::time::Duration;

use once_cell::sync::OnceCell;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::constants::{INIT_DELAY_SEC, MAX_DELAY_SEC, MAX_RETRY_COUNT};

mod cosmos;
mod ibc;
mod query;
mod tx;

pub use cosmos::*;
pub use ibc::*;
pub use query::*;
pub use tx::*;

pub fn base_dir() -> PathBuf {
    std::env::current_dir().unwrap().join("base")
}

pub static GLOBAL_SEED: OnceCell<u64> = OnceCell::new();
static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static THREAD_ID: RefCell<usize> = RefCell::new(THREAD_COUNTER.fetch_add(1, Ordering::SeqCst));

    static THREAD_RNG: RefCell<SmallRng> = {
        // u64 ? [u8; 32] ???
        let seed = GLOBAL_SEED.get().expect("Seed must be initialized first");
        let mut seed_bytes = [0u8; 32];
        seed_bytes[..8].copy_from_slice(&seed.to_le_bytes());

        RefCell::new(SmallRng::from_seed(seed_bytes))
    };
}

pub fn with_rng<F, R>(f: F) -> R
where
    F: FnOnce(&mut SmallRng) -> R,
{
    THREAD_RNG.with(|rng| f(&mut rng.borrow_mut()))
}

pub fn thread_id() -> usize {
    THREAD_ID.with(|id| *id.borrow())
}

pub type RetryConfig = RetryFutureConfig<ExponentialBackoff, NoOnRetry>;

pub fn retry_config() -> RetryConfig {
    RetryFutureConfig::new(MAX_RETRY_COUNT)
        .exponential_backoff(Duration::from_secs(INIT_DELAY_SEC))
        .max_delay(Duration::from_secs(MAX_DELAY_SEC))
}
