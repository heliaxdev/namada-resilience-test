use std::collections::BTreeSet;

use crate::sdk::namada::Sdk;
use crate::{
    entities::Alias,
    executor::StepError,
    state::State,
    step::StepContext,
    task::{self, Task, TaskSettings},
};

use super::utils;

#[derive(Debug, Default)]
pub struct InitAccount;

impl StepContext for InitAccount {
    fn name(&self) -> String {
        "init-account".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.min_n_implicit_accounts(3))
    }

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
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
            task::init_account::InitAccount::builder()
                .target(account_alias)
                .sources(source_aliases)
                .threshold(required_signers)
                .settings(task_settings)
                .build(),
        )])
    }
}
