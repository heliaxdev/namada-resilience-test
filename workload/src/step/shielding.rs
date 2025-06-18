use crate::constants::{COSMOS_TOKEN, MAX_BATCH_TX_NUM, MIN_TRANSFER_BALANCE};
use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::Alias;
use crate::utils::{get_masp_epoch, ibc_denom, is_native_denom, retry_config};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Shielding;

impl StepContext for Shielding {
    fn name(&self) -> String {
        "shielding".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_account_with_min_balance(1, MIN_TRANSFER_BALANCE))
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
        let epoch = get_masp_epoch(ctx, retry_config()).await?;
        let target_account = state
            .random_payment_address(vec![])
            .ok_or(StepError::BuildTask("No more accounts".to_string()))?;
        let balance = if is_native_denom(&denom) {
            state.get_balance_for(&source_account.alias)
        } else {
            state.get_ibc_balance_for(&source_account.alias, &denom)
        };
        let amount = utils::random_between(1, balance / MAX_BATCH_TX_NUM);

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let task_settings = TaskSettings::new(source_account.public_keys, gas_payer);

        Ok(vec![Task::Shielding(
            task::shielding::Shielding::builder()
                .source(source_account.alias)
                .target(target_account.alias.payment_address())
                .denom(denom)
                .amount(amount)
                .epoch(epoch)
                .settings(task_settings)
                .build(),
        )])
    }
}
