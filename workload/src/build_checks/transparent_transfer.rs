use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, executor::StepError};

use super::utils::get_balance;

pub async fn transparent_transfer(
    sdk: &Sdk,
    source: &Alias,
    target: &Alias,
    amount: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<Vec<Check>, StepError> {
    let pre_balance = get_balance(sdk, source, retry_config).await?;
    let source_check = Check::BalanceSource(source.clone(), pre_balance, amount);

    let pre_balance = get_balance(sdk, target, retry_config).await?;
    let target_check = Check::BalanceTarget(target.clone(), pre_balance, amount);

    Ok(vec![source_check, target_check])
}
