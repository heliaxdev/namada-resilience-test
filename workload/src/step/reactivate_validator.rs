use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, retry_config};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ReactivateValidator;

impl StepContext for ReactivateValidator {
    fn name(&self) -> String {
        "reactivate-validator".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_deactivated_validator(1))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let epoch = get_epoch(ctx, retry_config()).await?;
        let Some(account) = state.random_deactivated_validator(vec![], epoch, 1).pop() else {
            return Ok(vec![]);
        };

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

        Ok(vec![Task::ReactivateValidator(
            task::reactivate_validator::ReactivateValidator::builder()
                .target(account.alias)
                .settings(task_settings)
                .build(),
        )])
    }
}
