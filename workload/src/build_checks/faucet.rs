use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk};

pub async fn faucet_build_check(
    sdk: &Sdk,
    target: &Alias,
    amount: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Vec<Check> {
    let check = if let Some(pre_balance) =
        super::utils::get_balance(sdk, target, retry_config).await
    {
        Check::BalanceTarget(target.clone(), pre_balance, amount)
    } else {
        return vec![];
    };

    vec![check]
}
