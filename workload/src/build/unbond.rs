use namada_sdk::rpc;

use crate::{
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    state::State,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_unbond(sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
    let source_bond = state.random_bond();
    let source_account = state.get_account_by_alias(&source_bond.alias);
    let amount = utils::random_between(state, 1, source_bond.amount);

    let current_epoch = rpc::query_epoch(&sdk.namada.client)
        .await
        .map_err(StepError::Rpc)?;

    let mut task_settings = TaskSettings::new(source_account.public_keys, Alias::faucet());
    task_settings.gas_limit *= 3;

    Ok(vec![Task::Unbond(
        source_account.alias,
        source_bond.validator,
        amount,
        current_epoch
            .checked_add(6)
            .expect("Epoch shouldn't overflow")
            .into(),
        task_settings,
    )])
}
