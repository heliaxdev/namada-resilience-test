use std::collections::BTreeSet;

use crate::{
    entities::Alias,
    executor::StepError,
    state::State,
    task::{Task, TaskSettings},
};

use super::utils;

pub async fn build_init_account(state: &mut State) -> Result<Vec<Task>, StepError> {
    let random_alias = utils::random_alias(state);
    let account_alias = Alias {
        name: format!("{}-enstablished", random_alias.name),
    };
    let total_signers = utils::random_between(state, 1, 4);
    let required_signers = utils::random_between(state, 1, total_signers);

    let source_aliases = state
        .random_implicit_accounts(vec![], total_signers as usize)
        .into_iter()
        .map(|account| account.alias)
        .collect::<BTreeSet<Alias>>();

    let task_settings = TaskSettings::new(source_aliases.clone(), Alias::faucet());

    Ok(vec![Task::InitAccount(
        account_alias,
        source_aliases,
        required_signers,
        task_settings,
    )])
}
