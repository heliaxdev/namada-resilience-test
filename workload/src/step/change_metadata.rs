use async_trait::async_trait;

use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct ChangeMetadata;

#[async_trait]
impl StepContext for ChangeMetadata {
    fn name(&self) -> String {
        "change-metadata".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_validators(1))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let website = utils::get_random_string(state, 15);
        let email = format!("{}@test.com", utils::get_random_string(state, 5));
        let discord = utils::get_random_string(state, 10);
        let description = utils::get_random_string(state, 30);
        let avatar = utils::get_random_string(state, 20);

        let task_settings = TaskSettings::new(account.public_keys.clone(), Alias::faucet());

        Ok(vec![Task::ChangeMetadata(
            task::change_metadata::ChangeMetadata::builder()
                .source(account.alias)
                .website(website)
                .email(email)
                .discord(discord)
                .description(description)
                .avatar(avatar)
                .settings(task_settings)
                .build(),
        )])
    }
}
