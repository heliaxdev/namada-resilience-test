use crate::constants::NATIVE_SCALE;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};

#[derive(Clone, Debug, Default)]
pub struct FaucetTransfer;

impl StepContext for FaucetTransfer {
    fn name(&self) -> String {
        "faucet-transfer".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_account())
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let target_account = state
            .random_account(vec![])
            .ok_or(StepError::Build("No more accounts".to_string()))?;
        let amount = 1_000_000 * NATIVE_SCALE;

        let task_settings = TaskSettings::faucet();

        Ok(vec![Task::FaucetTransfer(
            task::faucet_transfer::FaucetTransfer::builder()
                .target(target_account.alias)
                .amount(amount)
                .settings(task_settings)
                .build(),
        )])
    }
}
