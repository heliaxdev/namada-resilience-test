use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk};

pub async fn shielded_transfer(
    sdk: &Sdk,
    source: Alias,
    target: Alias,
    amount: u64,
    with_indexer: bool,
    _retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Vec<Check> {
    let source_check = if let Ok(Some(pre_balance)) =
        super::utils::get_shielded_balance(sdk, source.clone(), None, with_indexer).await
    {
        Check::BalanceShieldedSource(source, pre_balance, amount)
    } else {
        return vec![];
    };

    let target_check = if let Ok(Some(pre_balance)) =
        super::utils::get_shielded_balance(sdk, target.clone(), None, with_indexer).await
    {
        Check::BalanceShieldedTarget(target, pre_balance, amount)
    } else {
        return vec![source_check];
    };

    vec![source_check, target_check]
}
