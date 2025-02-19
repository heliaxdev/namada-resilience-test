use crate::sdk::namada::Sdk;
use crate::{
    entities::Alias,
    executor::StepError,
    state::State,
    step::StepContext,
    task::{self, Task, TaskSettings},
};

#[derive(Debug, Default)]
pub struct ReactivateValidator;

impl StepContext for ReactivateValidator {
    fn name(&self) -> String {
        "reactivate-validator".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_deactivated_validators(1))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let account = state.random_deactivated_validator(vec![], 1).pop().unwrap();

        let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

        Ok(vec![Task::ReactivateValidator(
            task::reactivate_validator::ReactivateValidator::builder()
                .target(account.alias)
                .settings(task_settings)
                .build(),
        )])
    }
}
