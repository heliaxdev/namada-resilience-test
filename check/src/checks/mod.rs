use std::time::Duration;

use tokio::time::sleep;

use crate::sdk::namada::Sdk;

pub mod epoch;
pub mod height;
pub mod inflation;

pub trait DoCheck {
    async fn check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String>;
    async fn do_check(sdk: &Sdk, state: &mut crate::state::State) -> Result<(), String> {
        let mut times = 3;
        while times > 0 {
            let result = Self::check(sdk, state).await;
            if result.is_ok() {
                return result
            } else {
                if times == 1 {
                    tracing::info!("Check {} failed {} times, returning error", Self::to_string(), times);
                    return result
                }
                tracing::info!("Check {} failed retrying ({}/{})...", Self::to_string(), 3 - times, 3);
                times = times - 1;
                sleep(Duration::from_secs(1)).await
            }
        }
        Err(format!("Failed {} check", Self::to_string()))
    }
    
    fn to_string() -> String;
}
