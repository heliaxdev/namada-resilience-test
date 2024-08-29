use namada_sdk::{rpc, Namada};

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct InflationCheck {}

impl DoCheck for InflationCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.client();
        let native_token = match rpc::query_native_token(client).await {
            Ok(address) => address,
            Err(e) => {
                return Err(e.to_string());
            }
        };

        let total_supply = rpc::get_token_total_supply(client, &native_token).await;

        match total_supply {
            Ok(current_total_supply) => {
                if state.last_total_supply <= current_total_supply {
                    state.last_total_supply = current_total_supply;
                    tracing::info!("Total supply ok");
                    Ok(())
                } else {
                    Err(format!(
                        "Total supply decreases: before: {} -> after {}",
                        state.last_total_supply, current_total_supply
                    ))
                }
            }
            Err(e) => Err(format!("Failed to query total supply: {}", e)),
        }
    }

    fn timing() -> u32 {
        30
    }

    fn to_string() -> String {
        "InflationCheck".to_string()
    }
}
