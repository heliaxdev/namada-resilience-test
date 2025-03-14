use antithesis_sdk::random::AntithesisRng;
use rand::seq::IteratorRandom;
use serde_json::json;

use crate::code::Code;
use crate::constants::{FAUCET_AMOUNT, INIT_ESTABLISHED_ADDR_NUM, INIT_IMPLICIT_ADDR_NUM};
use crate::error::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::{StepContext, StepType};
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::utils::{get_epoch, get_validator_addresses, retry_config};
use crate::{assert_always_step, assert_unrechable_step};

use super::utils;

/// Initialize accounts. Use this with `--no-check`.
#[derive(Clone, Debug, Default)]
pub struct Initialize;

impl StepContext for Initialize {
    fn name(&self) -> String {
        "initialize".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.stats.is_empty())
    }

    async fn build_task(&self, sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let mut tasks = vec![];
        let mut implicit_aliases = vec![];
        let mut established_aliases = vec![];

        // implicit and shieled accounts
        let mut batch_tasks = vec![];
        for _ in 0..INIT_IMPLICIT_ADDR_NUM {
            let step_type = StepType::NewWalletKeyPair(Default::default());
            let task = Box::pin(step_type.build_task(sdk, state)).await?.remove(0);
            if let Task::NewWalletKeyPair(ref inner) = task {
                implicit_aliases.push(inner.source().clone());
                batch_tasks.push(task);
            }
        }
        let settings = TaskSettings::faucet_batch(INIT_IMPLICIT_ADDR_NUM as usize);
        tasks.push(Task::Batch(
            task::batch::Batch::builder()
                .tasks(batch_tasks)
                .settings(settings)
                .build(),
        ));

        // established accounts
        let task_settings = TaskSettings::faucet();
        for _ in 0..INIT_ESTABLISHED_ADDR_NUM {
            let alias = utils::random_alias();
            let account_alias = alias.established();
            established_aliases.push(account_alias.clone());

            let total_signers = utils::random_between(1, 4);
            let required_signers = utils::random_between(1, total_signers);
            let source_aliases = implicit_aliases
                .clone()
                .into_iter()
                .choose_multiple(&mut AntithesisRng, total_signers as usize)
                .into_iter()
                .collect();
            // avoid batching them to save accounts to the wallet
            tasks.push(Task::InitAccount(
                task::init_account::InitAccount::builder()
                    .target(account_alias)
                    .sources(source_aliases)
                    .threshold(required_signers)
                    .settings(task_settings.clone())
                    .build(),
            ));
        }

        // faucet transafer to all created addresses
        let batch_tasks = implicit_aliases
            .iter()
            .map(|alias| {
                Task::FaucetTransfer(
                    task::faucet_transfer::FaucetTransfer::builder()
                        .target(alias.clone())
                        .amount(FAUCET_AMOUNT)
                        .settings(task_settings.clone())
                        .build(),
                )
            })
            .collect();
        let settings = TaskSettings::faucet_batch(INIT_IMPLICIT_ADDR_NUM as usize);
        tasks.push(Task::Batch(
            task::batch::Batch::builder()
                .tasks(batch_tasks)
                .settings(settings)
                .build(),
        ));

        // bond
        let current_epoch = get_epoch(sdk, retry_config()).await?;
        let validators = get_validator_addresses(sdk, retry_config()).await?;
        let mut batch_tasks = vec![];
        for alias in implicit_aliases {
            let amount = utils::random_between(1, FAUCET_AMOUNT);

            let validator = validators
                .iter()
                .choose(&mut AntithesisRng)
                .expect("There is always at least a validator");

            let task_settings = TaskSettings::new([alias.clone()].into(), Alias::faucet());

            batch_tasks.push(Task::Bond(
                task::bond::Bond::builder()
                    .source(alias)
                    .validator(validator.to_string())
                    .amount(amount)
                    .epoch(current_epoch)
                    .settings(task_settings.clone())
                    .build(),
            ));
        }
        let settings = TaskSettings::faucet_batch(INIT_IMPLICIT_ADDR_NUM as usize);
        tasks.push(Task::Batch(
            task::batch::Batch::builder()
                .tasks(batch_tasks)
                .settings(settings)
                .build(),
        ));

        Ok(tasks)
    }

    fn assert(&self, code: &Code) {
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_successful {
            assert_always_step!("Done Initialize", details)
        } else {
            assert_unrechable_step!("Fatal Initialize", details)
        }
    }
}
