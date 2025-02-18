use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, constants::PROPOSAL_DEPOSIT, entities::Alias, sdk::namada::Sdk, executor::StepError};

use super::utils::get_balance;

pub async fn proposal(
    sdk: &Sdk,
    source: &Alias,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<Vec<Check>, StepError> {
    let pre_balance = get_balance(sdk, source, retry_config).await?;
    let source_check = Check::BalanceSource(source.clone(), pre_balance, PROPOSAL_DEPOSIT);

    Ok(vec![source_check])
}
