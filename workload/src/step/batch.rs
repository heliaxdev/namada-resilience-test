use std::pin::Pin;

use rand::seq::SliceRandom;

use crate::constants::MIN_TRANSFER_BALANCE;
use crate::{
    executor::StepError,
    sdk::namada::Sdk,
    state::State,
    step::{StepContext, StepType},
    task::{self, Task, TaskSettings},
};

#[derive(Debug, Default)]
pub struct BatchBond {
    max_size: u64,
}

impl StepContext for BatchBond {
    fn name(&self) -> String {
        "batch-bond".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        Box::pin(build_batch(
            sdk,
            vec![StepType::Bond(Default::default())],
            self.max_size,
            state,
        ))
        .await
    }
}

#[derive(Debug, Default)]
pub struct BatchRandom {
    max_size: u64,
}

impl StepContext for BatchRandom {
    fn name(&self) -> String {
        "batch-random".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_account_with_min_balance(3, MIN_TRANSFER_BALANCE) && state.min_bonds(3))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        Box::pin(build_batch(
            sdk,
            vec![
                StepType::TransparentTransfer(Default::default()),
                StepType::Bond(Default::default()),
                StepType::Redelegate(Default::default()),
                StepType::Unbond(Default::default()),
                StepType::Shielding(Default::default()),
                StepType::Unshielding(Default::default()),
                // StepType::ClaimRewards, introducing this would fail every balance check :(
            ],
            self.max_size,
            state,
        ))
        .await
    }
}

async fn build_batch(
    sdk: &Sdk,
    possibilities: Vec<StepType>,
    max_size: u64,
    state: &mut State,
) -> Result<Vec<Task>, StepError> {
    let mut tmp_state = state.clone();

    let mut batch_tasks = vec![];
    for _ in 0..max_size {
        let step = possibilities
            .choose(&mut state.rng)
            .expect("at least one StepType should exist");
        let tasks = step.build_task(sdk, &mut tmp_state).await?;
        tmp_state.update(&tasks, false);
        if !tasks.is_empty() {
            tracing::info!("Added {:?} tx type to the batch...", step);
            batch_tasks.extend(tasks);
        }
    }

    if batch_tasks.is_empty() {
        return Err(StepError::EmptyBatch);
    }

    let settings = TaskSettings::faucet_batch(batch_tasks.len());

    Ok(vec![Task::Batch(
        task::batch::Batch::builder()
            .tasks(batch_tasks)
            .settings(settings)
            .build(),
    )])
}
