use crate::constants::{COSMOS_TOKEN, MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::utils::{ibc_denom, is_native_denom};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct TransparentTransfer;

impl StepContext for TransparentTransfer {
    fn name(&self) -> String {
        "transparent-transfer".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, _state: &State) -> Result<bool, StepError> {
        Ok(true)
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let (source_account, denom) = state
            .random_account_with_ibc_balance(vec![])
            .filter(|_| utils::coin_flip(0.5))
            .map(|account| (account, ibc_denom(&ctx.namada_channel_id, COSMOS_TOKEN)))
            .or_else(|| {
                state
                    .random_account_with_min_balance(vec![], MIN_TRANSFER_BALANCE)
                    .map(|account| (account, Alias::nam().name))
            })
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let target_account = state
            .random_account(vec![source_account.alias.clone()])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let balance = if is_native_denom(&denom) {
            state.get_balance_for(&source_account.alias)
        } else {
            state.get_ibc_balance_for(&source_account.alias, &denom)
        };
        let amount = utils::random_between(1, balance / MAX_BATCH_TX_NUM);

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

        Ok(vec![Task::TransparentTransfer(
            task::transparent_transfer::TransparentTransfer::builder()
                .source(source_account.alias)
                .target(target_account.alias)
                .denom(denom)
                .amount(amount)
                .settings(task_settings)
                .build(),
        )])
    }
}
