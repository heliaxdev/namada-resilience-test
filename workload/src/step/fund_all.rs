use serde_json::json;

use crate::code::Code;
use crate::constants::FAUCET_AMOUNT;
use crate::error::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

#[derive(Clone, Debug, Default)]
pub struct FundAll;

impl StepContext for FundAll {
    fn name(&self) -> String {
        "fund-all".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(!state.accounts.is_empty())
    }

    async fn build_task(&self, _sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let tasks: Vec<_> = state
            .accounts
            .keys()
            .map(|alias| {
                Task::FaucetTransfer(
                    task::faucet_transfer::FaucetTransfer::builder()
                        .target(alias.clone())
                        .amount(FAUCET_AMOUNT)
                        .settings(TaskSettings::faucet())
                        .build(),
                )
            })
            .collect();

        let settings = TaskSettings::faucet_batch(tasks.len());
        Ok(vec![Task::Batch(
            task::batch::Batch::builder()
                .tasks(tasks)
                .settings(settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal FundAll", details)
        } else if is_successful {
            assert_always_step!("Done FundAll", details)
        } else {
            assert_sometimes_step!("Failed FundAll", details)
        }
    }
}
