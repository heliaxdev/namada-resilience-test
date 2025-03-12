use std::collections::HashMap;

use namada_sdk::rpc;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::types::{Alias, Fee};
use crate::utils::RetryConfig;

#[derive(TypedBuilder)]
pub struct RevealPk {
    target: Alias,
}

impl CheckContext for RevealPk {
    fn summary(&self) -> String {
        format!("reveal/{}", self.target.name)
    }

    async fn do_check(
        &self,
        sdk: &Sdk,
        _fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let target = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| StepError::Wallet(format!("No target address: {}", self.target.name)))?
            .into_owned();
        drop(wallet);

        let public_key = target.to_pretty_string();
        match tryhard::retry_fn(|| rpc::is_public_key_revealed(&sdk.namada.client, &target))
            .with_config(retry_config)
            .await
        {
            Ok(was_pk_revealed) => {
                antithesis_sdk::assert_always!(
                    was_pk_revealed,
                    "The public key was revealed correctly",
                    &json!({
                        "public_key": public_key,
                        "execution_height": check_info.execution_height,
                        "check_height": check_info.check_height,
                    })
                );
                if was_pk_revealed {
                    Ok(())
                } else {
                    Err(StepError::StateCheck(format!(
                        "RevealPk check error: pk for {public_key} was not revealed",
                    )))
                }
            }
            Err(e) => {
                tracing::error!(
                    "{}",
                    json!({
                        "public_key": public_key,
                        "execution_height": check_info.execution_height,
                        "check_height": check_info.check_height,
                    })
                );
                Err(StepError::StateCheck(format!("RevealPk check error: {e}")))
            }
        }
    }
}
