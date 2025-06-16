use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, get_rewards, retry_config};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ClaimRewards;

impl StepContext for ClaimRewards {
    fn name(&self) -> String {
        "claim-rewards".to_string()
    }

    async fn is_valid(&self, _ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        Ok(state.at_least_bond(1))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let epoch = get_epoch(ctx, retry_config()).await?;
        let Some(source_bond) = state.random_bond(epoch) else {
            return Ok(vec![]);
        };
        let source_account = state.get_account_by_alias(&source_bond.alias);

        // Need the reward amount for the state updating
        let already_claimed = state
            .get_claimed_epoch(&source_bond.alias)
            .is_some_and(|claimed_epoch| claimed_epoch >= epoch);
        let reward_amount = if already_claimed {
            0u64
        } else {
            let rewards = get_rewards(
                ctx,
                &source_bond.alias,
                &source_bond.validator,
                epoch,
                retry_config(),
            )
            .await?;
            rewards
                .to_string()
                .parse()
                .expect("Amount conversion shouldn't fail")
        };

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let mut task_settings = TaskSettings::new(source_account.public_keys, gas_payer);
        task_settings.gas_limit *= 5;

        Ok(vec![Task::ClaimRewards(
            task::claim_rewards::ClaimRewards::builder()
                .source(source_bond.alias)
                .from_validator(source_bond.validator.to_string())
                .amount(reward_amount)
                .settings(task_settings)
                .build(),
        )])
    }
}
