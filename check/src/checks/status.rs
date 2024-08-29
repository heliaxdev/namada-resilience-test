use namada_sdk::{queries::Client, Namada};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct StatusCheck {}

impl DoCheck for StatusCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.client();
        let status = client.status().await;

        match status {
            Ok(status) => {
                tracing::info!("Node moniker: {}", status.node_info.moniker);
                tracing::info!(
                    "Node voting power: {}",
                    status.validator_info.power.to_string()
                );
                tracing::info!("Node is catching up: {}", status.sync_info.catching_up);
                Ok(())
            }
            Err(e) => Err(format!("Failed to query status: {}", e)),
        }
    }

    fn timing() -> u64 {
        2
    }

    fn to_string() -> String {
        "StatusCheck".to_string()
    }
}
