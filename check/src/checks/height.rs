use tendermint_rpc::Client;

use crate::sdk::namada::Sdk;

use super::DoCheck;

#[derive(Clone, Debug, Default)]
pub struct HeightCheck {}

impl DoCheck for HeightCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let client = sdk.namada.clone_client();
        let last_block = client.latest_block().await;

        match last_block {
            Ok(block) => {
                let current_block_height = u64::from(block.block.header.height);
                if state.last_block_height <= current_block_height {
                    if state.last_block_height == current_block_height {
                        state.total_times_block_query_was_equal += 1;
                        if state.total_times_block_query_was_equal > 3 {
                            return Err(format!(
                                "Block height didn't change for 3 times: before: {} -> after {}, times {}",
                                state.last_block_height, current_block_height, state.total_times_block_query_was_equal
                            ));
                        }
                    } else {
                        state.last_block_height = current_block_height;
                        state.total_times_block_query_was_equal = 0;
                    }

                    tracing::info!("Block height ok");
                    Ok(())
                } else {
                    Err(format!(
                        "Block height didnt increase: before: {} -> after {}",
                        state.last_block_height, current_block_height
                    ))
                }
            }
            Err(e) => Err(format!("Failed to query last block: {}", e)),
        }
    }

    fn timing() -> u32 {
        10
    }

    fn to_string() -> String {
        "HeightCheck".to_string()
    }
}
