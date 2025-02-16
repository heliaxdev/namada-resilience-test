use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk};

pub async fn bond(
    sdk: &Sdk,
    source: &Alias,
    validator: &str,
    amount: u64,
    epoch: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Vec<Check> {
    let bond_check = if let Some(pre_bond) =
        super::utils::get_bond(sdk, source, validator, epoch, retry_config).await
    {
        Check::BondIncrease(source.clone(), validator.to_string(), pre_bond, amount)
    } else {
        return vec![];
    };
    vec![bond_check]
}
