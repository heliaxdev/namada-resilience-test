use rand::seq::IteratorRandom;

use crate::constants::MAX_BATCH_TX_NUM;
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, get_validator_addresses, retry_config, with_rng};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Redelegate;

impl StepContext for Redelegate {
    fn name(&self) -> String {
        "redelegate".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.any_bond())
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let current_epoch = get_epoch(ctx, retry_config()).await?;
        let Some(source_bond) = state.random_bond(current_epoch) else {
            return Ok(vec![]);
        };
        let source_account = state.get_account_by_alias(&source_bond.alias);
        let amount = utils::random_between(1, source_bond.amount / MAX_BATCH_TX_NUM);

        let validators = get_validator_addresses(ctx, retry_config()).await?;

        let source_redelegations = state.get_redelegations_targets_for(&source_account.alias);
        if source_redelegations.contains(&source_bond.validator) {
            return Ok(vec![]);
        }

        let to_validator = if let Some(validator) = with_rng(|rng| {
            validators
                .iter()
                .filter(|v| v.to_string() != source_bond.validator)
                .choose(rng)
        }) {
            validator
        } else {
            return Ok(vec![]);
        };

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let mut task_settings = TaskSettings::new(source_account.public_keys, gas_payer);
        task_settings.gas_limit *= 5;

        Ok(vec![Task::Redelegate(
            task::redelegate::Redelegate::builder()
                .source(source_account.alias)
                .from_validator(source_bond.validator.to_string())
                .to_validator(to_validator.to_string())
                .amount(amount)
                .epoch(current_epoch)
                .settings(task_settings)
                .build(),
        )])
    }
}
