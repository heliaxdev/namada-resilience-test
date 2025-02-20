use std::time::Duration;

use rand::Rng;
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::state::State;

mod query;
mod tx;

pub use query::*;
pub use tx::*;

pub fn get_random_between(state: &mut State, min: u64, max: u64) -> u64 {
    state.rng.gen_range(min..max)
}

pub type RetryConfig = RetryFutureConfig<ExponentialBackoff, NoOnRetry>;

pub fn retry_config() -> RetryConfig {
    RetryFutureConfig::new(4)
        .exponential_backoff(Duration::from_secs(1))
        .max_delay(Duration::from_secs(10))
}
