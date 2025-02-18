use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, executor::StepError};

use super::utils::get_bond;

pub async fn bond(
    sdk: &Sdk,
    source: &Alias,
    validator: &str,
    amount: u64,
    epoch: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<Vec<Check>, StepError> {
    let pre_bond = get_bond(sdk, source, validator, epoch, retry_config).await?;

    Ok(vec![Check::BondIncrease(source.clone(), validator.to_string(), pre_bond, amount)])
}
