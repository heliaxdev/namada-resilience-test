use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ChangeMetadata;

impl StepContext for ChangeMetadata {
    fn name(&self) -> String {
        "change-metadata".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_validator(1))
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_validator(vec![], 1).pop().unwrap();

        let website = utils::get_random_string(15);
        let email = format!("{}@test.com", utils::get_random_string(5));
        let discord = utils::get_random_string(10);
        let description = utils::get_random_string(30);
        let avatar = utils::get_random_string(20);

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

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
