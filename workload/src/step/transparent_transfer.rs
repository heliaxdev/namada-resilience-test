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
pub struct TransparentTransfer;

impl StepContext for TransparentTransfer {
    fn name(&self) -> String {
        "transparent-transfer".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_accounts(2) && state.any_account_can_make_transfer())
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let source_account = state
            .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
            .ok_or(StepError::Build("No more accounts".to_string()))?;
        let target_account = state
            .random_account(vec![source_account.alias.clone()])
            .ok_or(StepError::Build("No more accounts".to_string()))?;
        let amount_account = state.get_balance_for(&source_account.alias);
        let amount = utils::random_between(state, 1, amount_account);

        let task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());

        Ok(vec![Task::TransparentTransfer(
            task::transparent_transfer::TransparentTransfer::builder()
                .source(source_account.alias)
                .target(target_account.alias)
                .amount(amount)
                .settings(task_settings)
                .build(),
        )])
    }
}
