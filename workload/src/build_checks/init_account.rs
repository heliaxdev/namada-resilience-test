use std::collections::BTreeSet;

use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk};

pub async fn init_account(
    alias: &Alias,
    sources: &BTreeSet<Alias>,
    threshold: u64,
) -> Vec<Check> {
    vec![Check::AccountExist(alias.clone(), threshold, sources.clone())]
}
