use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{check::Check, entities::Alias, sdk::namada::Sdk, executor::StepError};

use super::utils::get_bond;

#[allow(clippy::too_many_arguments)]
pub async fn redelegate(
    sdk: &Sdk,
    source: &Alias,
    from_validator: &str,
    to_validator: &str,
    amount: u64,
    epoch: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<Vec<Check>, StepError> {
    let pre_bond = get_bond(
        sdk,
        source,
        from_validator,
        epoch,
        retry_config,
    )
    .await?;
    let from_validator_bond_check = Check::BondDecrease(source.clone(), from_validator.to_string(), pre_bond, amount);

    let pre_bond = get_bond(
        sdk,
        source,
        to_validator,
        epoch,
        retry_config,
    )
    .await?;

    let to_validator_bond_check = Check::BondIncrease(source.clone(), to_validator.to_string(), pre_bond, amount);

    Ok(vec![from_validator_bond_check, to_validator_bond_check])
}
