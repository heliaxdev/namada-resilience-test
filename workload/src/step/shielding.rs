use crate::sdk::namada::Sdk;
use crate::{
    constants::MIN_TRANSFER_BALANCE,
    entities::Alias,
    executor::StepError,
    state::State,
    step::StepContext,
    task::{self, Task, TaskSettings},
};

use super::utils;

#[derive(Debug, Default)]
pub struct Shielding;

impl StepContext for Shielding {
    fn name(&self) -> String {
        "shielding".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account_with_min_balance(MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::Build("No more accounts".to_string()))?;
        let target_account = state
            .random_payment_address(vec![])
            .ok_or(StepError::Build("No more accounts".to_string()))?;
        let amount_account = state.get_balance_for(&source_account.alias);
        let amount = utils::random_between(state, 1, amount_account);

        let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

        Ok(vec![Task::Shielding(
            task::shielding::Shielding::builder()
                .source(source_account.alias)
                .target(target_account.payment_address)
                .amount(amount)
                .settings(task_settings)
                .build(),
        )])
    }
}
