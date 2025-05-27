use crate::constants::FAUCET_AMOUNT;
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct FundAll;

impl StepContext for FundAll {
    fn name(&self) -> String {
        "fund-all".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(!state.accounts.is_empty())
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
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
}
