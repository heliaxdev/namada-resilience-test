use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, executor::StepError};

use super::utils::get_balance;

pub async fn faucet(
    sdk: &Sdk,
    target: &Alias,
    amount: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<Vec<Check>, StepError> {
    let pre_balance = get_balance(sdk, target, retry_config).await?;

    Ok(vec![Check::BalanceTarget(target.clone(), pre_balance, amount)])
}
