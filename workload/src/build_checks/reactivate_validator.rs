use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{
    check::{Check, ValidatorStatus},
    entities::Alias,
    sdk::namada::Sdk,
};

pub async fn reactivate_validator_build_checks(
    _sdk: &Sdk,
    alias: Alias,
    _retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Vec<Check> {
    vec![Check::ValidatorStatus(alias, ValidatorStatus::Reactivating)]
}
