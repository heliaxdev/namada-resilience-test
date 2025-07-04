use std::collections::BTreeSet;

use crate::constants::{COSMOS_TOKEN, DEFAULT_FEE, MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::utils::coin_flip;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::utils::{get_masp_epoch, ibc_denom, is_native_denom, retry_config};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Unshielding;

impl StepContext for Unshielding {
    fn name(&self) -> String {
        "unshielding".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_masp_account_with_minimal_balance(1, MIN_TRANSFER_BALANCE))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let Some((source_account, denom)) = state
            .random_masp_account_with_ibc_balance(vec![])
            .filter(|_| utils::coin_flip(0.5))
            .map(|account| (account, ibc_denom(&ctx.namada_channel_id, COSMOS_TOKEN)))
            .or_else(|| {
                state
                    .random_masp_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
                    .map(|account| (account, Alias::nam().name))
            })
        else {
            return Ok(vec![]);
        };

        let epoch = get_masp_epoch(ctx, retry_config()).await?;
        let target_account = state
            .random_account(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let balance = if is_native_denom(&denom) {
            state.get_shielded_balance_for(&source_account.alias)
        } else {
            state.get_ibc_balance_for(&source_account.alias.spending_key(), &denom)
        };
        let amount = utils::random_between(1, balance / MAX_BATCH_TX_NUM);

        let disposable_gas_payer = match (
            is_native_denom(&denom),
            state.get_balance_for(&source_account.alias.base()),
            state.get_shielded_balance_for(&source_account.alias),
        ) {
            (true, balance, _) if balance < DEFAULT_FEE => true,
            (true, _, _) => coin_flip(0.5),
            (_, balance, shielded_balance)
                if balance >= DEFAULT_FEE && shielded_balance >= DEFAULT_FEE =>
            {
                coin_flip(0.5)
            }
            (_, balance, _) if balance >= DEFAULT_FEE => false,
            (_, _, shielded_balance) if shielded_balance >= DEFAULT_FEE => true,
            _ => return Ok(vec![]), // insufficient fee
        };
        let task_settings = TaskSettings::new(
            BTreeSet::from([source_account.alias.base()]),
            if disposable_gas_payer {
                source_account.alias.spending_key()
            } else {
                source_account.alias.base()
            },
        );

        Ok(vec![Task::Unshielding(
            task::unshielding::Unshielding::builder()
                .source(source_account.alias.spending_key())
                .target(target_account.alias)
                .denom(denom)
                .amount(amount)
                .epoch(epoch)
                .settings(task_settings)
                .build(),
        )])
    }
}
