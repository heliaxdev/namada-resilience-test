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
pub struct ShieldedTransfer;

impl StepContext for ShieldedTransfer {
    fn name(&self) -> String {
        "shielded-transfer".to_string()
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
            .random_payment_address(vec![source_account.alias.clone()])
            .ok_or(StepError::BuildTask("No more target accounts".to_string()))?;
        let balance = if is_native_denom(&denom) {
            state.get_shielded_balance_for(&source_account.alias)
        } else {
            state.get_ibc_balance_for(&source_account.alias.spending_key(), &denom)
        };
        let amount = utils::random_between(1, balance / MAX_BATCH_TX_NUM);

        let transparent_source_balance = state.get_balance_for(&source_account.alias.base());
        let disposable_gas_payer = transparent_source_balance < DEFAULT_FEE || coin_flip(0.5);
        let task_settings = TaskSettings::new(
            BTreeSet::from([source_account.alias.base()]),
            if disposable_gas_payer {
                source_account.alias.spending_key()
            } else {
                source_account.alias.base()
            },
        );

        Ok(vec![Task::ShieldedTransfer(
            task::shielded::ShieldedTransfer::builder()
                .source(source_account.alias.spending_key())
                .target(target_account.alias.payment_address())
                .denom(denom)
                .amount(amount)
                .epoch(epoch)
                .settings(task_settings)
                .build(),
        )])
    }
}
