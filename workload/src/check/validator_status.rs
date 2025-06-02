use std::collections::HashMap;

use namada_sdk::proof_of_stake::types::ValidatorState;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Fee, ValidatorStatus as Status};
use crate::utils::{get_epoch, get_validator_state, RetryConfig};

#[derive(TypedBuilder)]
pub struct ValidatorStatus {
    target: Alias,
    status: Status,
}

impl CheckContext for ValidatorStatus {
    fn summary(&self) -> String {
        format!("validator-status/{}/{}", self.target.name, self.status)
    }

    async fn do_check(
        &self,
        ctx: &Ctx,
        _fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError> {
        let epoch = get_epoch(ctx, retry_config).await?;
        let (target_address, (state, _epoch)) =
            get_validator_state(ctx, &self.target, epoch + 2, retry_config).await?;
        let state = state.ok_or_else(|| {
            let details = json!({
                "target_alias": self.target,
                "target": target_address.to_pretty_string(),
                "execution_height": check_info.execution_height,
                "check_height": check_info.check_height
            });
            tracing::error!("OnChain validator account doesn't exist: {details}");
            CheckError::State(format!(
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

        if is_valid_status {
            tracing::info!("Validator status correctly changed: {details}");
            Ok(())
        } else {
            tracing::error!("Validator status is wrong: {details}");
            Err(CheckError::State(format!("ValidatorStatus check error: post target state {state:?} doesn't correspond to the expected status {}", self.status)))
        }
    }
}
