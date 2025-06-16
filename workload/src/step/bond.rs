use rand::seq::IteratorRandom;

use crate::constants::{MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, get_validator_addresses, retry_config, with_rng};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Bond;

impl StepContext for Bond {
    fn name(&self) -> String {
        "bond".to_string()
    }

    async fn is_valid(&self, _sdk: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_account_with_min_balance(1, MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let amount_account = state.get_balance_for(&source_account.alias);
        let amount = utils::random_between(1, amount_account / MAX_BATCH_TX_NUM);

        let current_epoch = get_epoch(ctx, retry_config()).await?;
        let validators = get_validator_addresses(ctx, retry_config()).await?;

        let validator = with_rng(|rng| {
            validators
                .iter()
                .choose(rng)
                .expect("There is always at least a validator")
        });

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

        Ok(vec![Task::Bond(
            task::bond::Bond::builder()
                .source(source_account.alias)
                .validator(validator.to_string())
                .amount(amount)
                .epoch(current_epoch)
                .settings(task_settings)
                .build(),
        )])
    }
}
