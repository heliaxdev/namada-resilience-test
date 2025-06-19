use std::collections::BTreeSet;

use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct InitAccount;

impl StepContext for InitAccount {
    fn name(&self) -> String {
        "init-account".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, _state: &State) -> Result<bool, StepError> {
        Ok(true)
    }

    async fn build_task(&self, _ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let random_alias = utils::random_alias();
        let account_alias = random_alias.established();
        let total_signers = utils::random_between(1, 4);
        let required_signers = utils::random_between(1, total_signers);

        let source_aliases = state
            .random_implicit_accounts(vec![], total_signers as usize)
            .into_iter()
            .map(|account| account.alias)
            .collect::<BTreeSet<Alias>>();

        let gas_payer = utils::get_gas_payer(source_aliases.iter(), state);
        let task_settings = TaskSettings::new(source_aliases.clone(), gas_payer);

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
