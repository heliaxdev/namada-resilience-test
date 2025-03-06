use std::time::Instant;

use namada_sdk::rpc;
use thiserror::Error;
use tokio::time::{sleep, Duration};

use crate::check::{Check, CheckContext, CheckInfo};
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::{StepContext, StepType};
use crate::task::{Task, TaskContext};
use crate::types::{Alias, Epoch, Height};
use crate::utils::{execute_reveal_pk, retry_config};

#[derive(Error, Debug)]
pub enum StepError {
    #[error("Building an empty batch")]
    EmptyBatch,
    #[error("Wallet failed: `{0}`")]
    Wallet(String),
    #[error("Building task failed: `{0}`")]
    BuildTask(String),
    #[error("Building tx failed: `{0}`")]
    BuildTx(String),
    #[error("Building check failed: `{0}`")]
    BuildCheck(String),
    #[error("Fetching shielded context data failed: `{0}`")]
    ShieldedSync(String),
    #[error("Broadcasting tx failed: `{0}`")]
    Broadcast(namada_sdk::error::Error),
    #[error("Executing tx failed: `{0}`")]
    Execution(String),
    #[error("Namada RPC request failed `{0}`")]
    Rpc(namada_sdk::error::Error),
    #[error("State check failed: `{0}`")]
    StateCheck(String),
    #[error("Shielded context failed: `{0}`")]
    ShieldedContext(String),
}

pub struct WorkloadExecutor {
    sdk: Sdk,
    state: State,
}

impl WorkloadExecutor {
    pub fn new(sdk: Sdk, state: State) -> Self {
        Self { sdk, state }
    }

    pub fn sdk(&self) -> &Sdk {
        &self.sdk
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub async fn fetch_current_block_height(&self) -> Height {
        loop {
            if let Ok(Some(latest_block)) = rpc::query_block(&self.sdk.namada.client).await {
                return latest_block.height.into();
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    async fn fetch_epoch_at_height(&self, height: Height) -> Epoch {
        loop {
            let epoch = rpc::query_epoch_at_height(&self.sdk.namada.client, height.into()).await;
            if let Ok(epoch) = epoch {
                return epoch.expect("Epoch should exist").into();
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    pub async fn init(&self) -> Result<(), StepError> {
        let client = &self.sdk.namada.client;
        let wallet = self.sdk.namada.wallet.read().await;
        let faucet_alias = Alias::faucet();
        let faucet_address = wallet
            .find_address(&faucet_alias.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", faucet_alias.name)))?
            .into_owned();
        let native_token_alias = Alias::nam();
        let nam_address = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?
            .into_owned();
        let faucet_public_key = wallet
            .find_public_key(&faucet_alias.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?
            .to_owned();
        drop(wallet);

        loop {
            if let Ok(res) =
                rpc::get_token_balance(client, &nam_address, &faucet_address, None).await
            {
                if res.is_zero() {
                    tracing::error!("Faucet has no money RIP.");
                    std::process::exit(1);
                } else {
                    tracing::info!("Faucet has $$$ ({})", res);
                    break;
                }
            }
            tracing::warn!("Retry querying for faucet balance...");
            sleep(Duration::from_secs(2)).await;
        }

        loop {
            if let Ok(is_revealed) = rpc::is_public_key_revealed(client, &faucet_address).await {
                if is_revealed {
                    break;
                }
            }
            if let Ok(Some(_)) = execute_reveal_pk(&self.sdk, faucet_public_key.clone()).await {
                break;
            }
            tracing::warn!("Retry revealing faucet pk...");
            sleep(Duration::from_secs(2)).await;
        }

        Ok(())
    }

    pub async fn is_valid(&self, step_type: &StepType) -> Result<bool, StepError> {
        step_type.is_valid(&self.sdk, &self.state).await
    }

    pub async fn build(&self, step_type: &StepType) -> Result<Vec<Task>, StepError> {
        step_type.build_task(&self.sdk, &self.state).await
    }

    pub async fn build_check(&self, tasks: &[Task]) -> Result<Vec<Check>, StepError> {
        let retry_config = retry_config();
        let mut checks = vec![];
        for task in tasks {
            let built_checks = task.build_checks(&self.sdk, retry_config).await?;
            built_checks
                .iter()
                .for_each(|check| check.assert_pre_balance(&self.state));
            checks.extend(built_checks)
        }
        Ok(checks)
    }

    pub async fn checks(
        &self,
        checks: Vec<Check>,
        execution_height: Option<u64>,
    ) -> Result<(), StepError> {
        let retry_config = retry_config();

        if checks.is_empty() {
            return Ok(());
        }

        let Some(execution_height) = execution_height else {
            return Ok(());
        };

        let check_height = self.fetch_current_block_height().await;
        for check in checks {
            tracing::info!("Running {check} check...");
            check
                .do_check(
                    &self.sdk,
                    CheckInfo {
                        execution_height,
                        check_height,
                    },
                    retry_config,
                )
                .await?;
        }

        Ok(())
    }

    pub async fn execute(&self, tasks: &[Task]) -> Result<Option<Height>, StepError> {
        let mut total_time = 0;
        let mut heights = vec![];

        for task in tasks {
            tracing::info!("Executing {task}...");
            let now = Instant::now();
            let execution_height = task.execute(&self.sdk).await?;

            total_time += now.elapsed().as_secs();
            heights.push(execution_height);
        }
        tracing::info!("Execution took {total_time}s...");

        let Some(execution_height) = heights.into_iter().flatten().max() else {
            return Ok(None);
        };
        // wait for the execution block settling
        loop {
            let current_height = self.fetch_current_block_height().await;
            if current_height > execution_height {
                break;
            } else {
                tracing::info!(
                    "Waiting for block height: {}, currently at: {}",
                    execution_height,
                    current_height
                );
            }
            sleep(Duration::from_secs(2)).await
        }

        Ok(Some(execution_height))
    }

    pub async fn post_execute(
        &mut self,
        tasks: &[Task],
        execution_height: Option<Height>,
    ) -> Result<(), StepError> {
        let Some(height) = execution_height else {
            return Ok(());
        };

        for task in tasks {
            // update state
            task.update_state(&mut self.state, true);
            task.update_stats(&mut self.state);

            match task {
                Task::ClaimRewards(cr) => {
                    // workaround for exact balance update after claim-rewards
                    let (_, balance) = crate::utils::get_balance(
                        &self.sdk,
                        cr.source(),
                        crate::utils::retry_config(),
                    )
                    .await?;
                    let balance = balance
                        .to_string()
                        .parse()
                        .expect("Balance conversion shouldn't fail");
                    self.state.overwrite_balance(cr.source(), balance);

                    let claimed_epoch = self.fetch_epoch_at_height(height).await;
                    self.state.set_claimed_epoch(cr.source(), claimed_epoch);
                }
                Task::InitAccount(_) => {
                    // save wallet for init-account
                    let wallet = self.sdk.namada.wallet.read().await;
                    wallet
                        .save()
                        .map_err(|e| StepError::Wallet(e.to_string()))?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn update_failed_execution(&mut self, tasks: &[Task]) {
        for task in tasks {
            task.update_failed_execution(&mut self.state);
        }
    }
}
