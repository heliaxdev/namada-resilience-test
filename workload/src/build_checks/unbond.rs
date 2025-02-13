use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk};

pub async fn unbond(
    sdk: &Sdk,
    source: Alias,
    validator: String,
    amount: u64,
    epoch: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Vec<Check> {
    let bond_check = if let Some(pre_bond) =
        super::utils::get_bond(sdk, source.clone(), validator.clone(), epoch, retry_config).await
    {
        Check::BondDecrease(source, validator, pre_bond, amount)
    } else {
        return vec![];
    };
    vec![bond_check]
}
