use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk};

pub async fn transparent_transfer(
    sdk: &Sdk,
    source: Alias,
    target: Alias,
    amount: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Vec<Check> {
    let source_check = if let Some(pre_balance) =
        super::utils::get_balance(sdk, source.clone(), retry_config).await
    {
        Check::BalanceSource(source, pre_balance, amount)
    } else {
        return vec![];
    };

    let target_check = if let Some(pre_balance) =
        super::utils::get_balance(sdk, target.clone(), retry_config).await
    {
        Check::BalanceTarget(target, pre_balance, amount)
    } else {
        return vec![source_check];
    };

    vec![source_check, target_check]
}
