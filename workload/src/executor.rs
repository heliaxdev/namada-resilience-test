use std::time::Instant;

use crate::check::{Check, CheckContext, CheckInfo};
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::{StepContext, StepType};
use crate::task::{Task, TaskContext};
use crate::types::Alias;
use crate::utils::{execute_reveal_pk, retry_config};
use namada_sdk::rpc;
use thiserror::Error;
use tokio::time::{sleep, Duration};

#[derive(Error, Debug)]
pub enum StepError {
    #[error("building an empty batch")]
    EmptyBatch,
    #[error("error wallet `{0}`")]
    Wallet(String),
    #[error("error building tx `{0}`")]
    Build(String),
    #[error("error fetching shielded context data `{0}`")]
    ShieldedSync(String),
    #[error("error broadcasting tx `{0}`")]
    Broadcast(String),
    #[error("error executing tx `{0}`")]
    Execution(String),
    #[error("error calling rpc `{0}`")]
    Rpc(namada_sdk::error::Error),
    #[error("build check: `{0}`")]
    BuildCheck(String),
    #[error("state check: `{0}`")]
    StateCheck(String),
}

#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub time_taken: u64,
    pub execution_height: Option<u64>,
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

    pub async fn fetch_current_block_height(&self) -> u64 {
        loop {
            if let Ok(Some(latest_block)) = rpc::query_block(&self.sdk.namada.client).await {
                return latest_block.height.into();
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    pub async fn fetch_current_epoch(&self) -> u64 {
        loop {
            let latest_epoch = rpc::query_epoch(&self.sdk.namada.client).await;
            if let Ok(epoch) = latest_epoch {
                return epoch.into();
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

    pub async fn build(&mut self, step_type: &StepType) -> Result<Vec<Task>, StepError> {
        step_type.build_task(&self.sdk, &mut self.state).await
    }

    pub async fn build_check(&self, tasks: &[Task]) -> Result<Vec<Check>, StepError> {
        let retry_config = retry_config();
        Ok(futures::future::try_join_all(
            tasks
                .iter()
                .map(|task| async move { task.build_checks(&self.sdk, retry_config).await }),
        )
        .await?
        .into_iter()
        .flatten()
        .collect())
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

        let check_height = loop {
            let current_height = self.fetch_current_block_height().await;
            if current_height >= execution_height {
                break current_height;
            } else {
                tracing::info!(
                    "Waiting for block height: {}, currently at: {}",
                    execution_height,
                    current_height
                );
            }
            sleep(Duration::from_secs(2)).await
        };

        for check in checks {
            tracing::info!("Running {} check...", check.to_string());
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

    pub async fn execute(&self, tasks: &[Task]) -> Result<Vec<ExecutionResult>, StepError> {
        let mut execution_results = vec![];

        for task in tasks {
            let now = Instant::now();
            let execution_height = task.execute(&self.sdk).await?;
            let execution_result = ExecutionResult {
                time_taken: now.elapsed().as_secs(),
                execution_height,
            };
            execution_results.push(execution_result);
        }

        Ok(execution_results)
    }

    pub fn update_state(&mut self, tasks: Vec<Task>) {
        for task in tasks {
            task.update_state(&mut self.state, true);
            task.update_stats(&mut self.state);
        }
    }

    pub fn update_failed_execution(&mut self, tasks: &[Task]) {
        for task in tasks {
            task.update_failed_execution(&mut self.state);
        }
    }
}
