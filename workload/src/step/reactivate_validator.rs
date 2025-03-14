use serde_json::json;

use crate::code::Code;
use crate::error::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct ReactivateValidator;

impl StepContext for ReactivateValidator {
    fn name(&self) -> String {
        "reactivate-validator".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_deactivated_validators(1))
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let account = state.random_deactivated_validator(vec![], 1).pop().unwrap();

        let gas_payer = utils::get_gas_payer(account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(account.public_keys, gas_payer);

        Ok(vec![Task::ReactivateValidator(
            task::reactivate_validator::ReactivateValidator::builder()
                .target(account.alias)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal ReactivateValidator", details)
        } else if is_successful {
            assert_always_step!("Done ReactivateValidator", details)
        } else {
            assert_sometimes_step!("Failed ReactivateValidator ", details)
        }
    }
}
