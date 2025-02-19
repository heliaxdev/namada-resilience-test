use crate::sdk::namada::Sdk;
use crate::{
    entities::Alias,
    executor::StepError,
    state::State,
    step::StepContext,
    task::{self, Task, TaskSettings},
};

#[derive(Debug, Default)]
pub struct DeactivateValidator;

impl StepContext for DeactivateValidator {
    fn name(&self) -> String {
        "deactivate-validator".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_validators(1))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

        Ok(vec![Task::DeactivateValidator(
            task::deactivate_validator::DeactivateValidator::builder()
                .target(account.alias)
                .settings(task_settings)
                .build(),
        )])
    }
}
