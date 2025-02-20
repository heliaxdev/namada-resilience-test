use async_trait::async_trait;
use namada_sdk::proof_of_stake::types::ValidatorState;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::types::{Alias, ValidatorStatus as Status};
use crate::utils::{get_epoch, get_validator_state, RetryConfig};

#[derive(TypedBuilder)]
pub struct ValidatorStatus {
    target: Alias,
    status: Status,
}

#[async_trait]
impl CheckContext for ValidatorStatus {
    fn summary(&self) -> String {
        format!("validator-status/{}/{}", self.target.name, self.status)
    }

    async fn do_check(
        &self,
        sdk: &Sdk,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), StepError> {
        let epoch = get_epoch(sdk, retry_config).await?;
        let (target_address, (state, _epoch)) =
            get_validator_state(sdk, &self.target, epoch + 2, retry_config).await?;
        let state = state.ok_or_else(|| {
            antithesis_sdk::assert_unreachable!(
                "OnChain validator account doesn't exist.",
                &json!({
                    "target_alias": self.target,
                    "target": target_address.to_pretty_string(),
                    "execution_height": check_info.execution_height,
                    "check_height": check_info.check_height
                })
            );
            StepError::StateCheck(format!(
                "ValidatorStatus check error: validator {} doesn't exist",
                self.target.name
            ))
        })?;

        let is_valid_status = match self.status {
            Status::Inactive => {
                matches!(state, ValidatorState::Inactive)
            }
            _ => !matches!(state, ValidatorState::Inactive),
        };
        let details = json!({
            "target_alias": self.target,
            "target": target_address.to_pretty_string(),
            "to_status": self.status.to_string(),
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height
        });

        antithesis_sdk::assert_always!(
            is_valid_status,
            "Validator status correctly changed.",
            &details
        );

        if is_valid_status {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(StepError::StateCheck(format!("ValidatorStatus check error: post target state {state:?} doesn't correspond to the expected status {}", self.status)))
        }
    }
}
