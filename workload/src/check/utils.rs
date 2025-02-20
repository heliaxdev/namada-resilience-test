use namada_sdk::rpc;
use tokio::time::{sleep, Duration};

use crate::sdk::namada::Sdk;

pub(super) async fn check_height(sdk: &Sdk, execution_height: u64) -> u64 {
    loop {
        if let Ok(Some(latest_block)) = rpc::query_block(&sdk.namada.client).await {
            let current_height = latest_block.height.into();
            if current_height >= execution_height {
                break current_height;
            } else {
                tracing::info!(
                    "Waiting for block height: {}, currently at: {}",
                    execution_height,
                    current_height
                );
            }
        }
        sleep(Duration::from_secs(2)).await
    }
}
