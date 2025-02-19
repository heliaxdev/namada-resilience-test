use crate::sdk::namada::Sdk;
use crate::{
    entities::Alias,
    executor::StepError,
    state::State,
    step::StepContext,
    task::{self, Task, TaskSettings},
};

#[derive(Debug, Default)]
pub struct ClaimRewards;

impl StepContext for ClaimRewards {
    fn name(&self) -> String {
        "claim-rewards".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_bond())
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);

        let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
        task_settings.gas_limit *= 5;

        Ok(vec![Task::ClaimRewards(
            task::claim_rewards::ClaimRewards::builder()
                .source(source_bond.alias)
                .from_validator(source_bond.validator.to_string())
                .settings(task_settings)
                .build(),
        )])
    }
}
