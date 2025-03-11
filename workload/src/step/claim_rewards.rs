use serde_json::json;

use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::utils::{get_epoch, get_rewards, retry_config};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

#[derive(Clone, Debug, Default)]
pub struct ClaimRewards;

impl StepContext for ClaimRewards {
    fn name(&self) -> String {
        "claim-rewards".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(state.any_bond())
    }

    async fn build_task(&self, sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);

        // Need the reward amount for the state updating
        let retry_config = retry_config();
        let epoch = get_epoch(sdk, retry_config).await?;
        let already_claimed = state
            .get_claimed_epoch(&source_bond.alias)
            .map_or(false, |claimed_epoch| claimed_epoch >= epoch);
        let reward_amount = if already_claimed {
            0u64
        } else {
            let rewards = get_rewards(
                sdk,
                &source_bond.alias,
                &source_bond.validator,
                epoch,
                retry_config,
            )
            .await?;
            rewards
                .to_string()
                .parse()
                .expect("Amount conversion shouldn't fail")
        };

        let mut task_settings = TaskSettings::new(source_account.public_keys);
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

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal ClaimRewards", details)
        } else if is_successful {
            assert_always_step!("Done ClaimRewards", details)
        } else {
            assert_sometimes_step!("Failed ClaimRewards ", details)
        }
    }
}
