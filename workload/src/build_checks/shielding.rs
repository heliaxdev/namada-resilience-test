use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, executor::StepError};

use super::utils::{get_balance, get_shielded_balance};

pub async fn shielding(
    sdk: &Sdk,
    source: &Alias,
    target: &Alias,
    amount: u64,
    with_indexer: bool,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<Vec<Check>, StepError> {
    let pre_balance = get_balance(sdk, source, retry_config).await?;
    let source_check = Check::BalanceSource(source.clone(), pre_balance, amount);

    let pre_balance = get_shielded_balance(sdk, target, None, with_indexer).await?.unwrap_or_default();
    let target_check = Check::BalanceShieldedTarget(target.clone(), pre_balance, amount);

    Ok(vec![source_check, target_check])
}
