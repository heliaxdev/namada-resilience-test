use std::collections::HashMap;
use std::time::Instant;

use namada_sdk::rpc;
use tokio::time::{sleep, Duration};

use crate::check::{Check, CheckContext, CheckInfo};
use crate::code::Code;
use crate::context::Ctx;
use crate::error::{CheckError, StepError, TaskError};
use crate::state::State;
use crate::stats::Stats;
use crate::step::{StepContext, StepType};
use crate::task::{Task, TaskContext};
use crate::types::{Alias, Epoch, Fee, Height};
use crate::utils::{
    execute_reveal_pk, get_block_height, get_proposals, is_pk_revealed, retry_config,
};

pub struct WorkloadExecutor {
    ctx: Ctx,
    state: State,
    stats: Stats,
    step_id: u64,
}

impl WorkloadExecutor {
    pub fn new(ctx: Ctx) -> Self {
        Self {
            ctx,
            state: State::new(),
            stats: Stats::default(),
            step_id: 0u64,
        }
    }

    pub fn final_report(self) -> Stats {
        self.stats
    }

    async fn fetch_epoch_at_height(&self, height: Height) -> Epoch {
        loop {
            let epoch = rpc::query_epoch_at_height(&self.ctx.namada.client, height.into()).await;
            if let Ok(epoch) = epoch {
                return epoch.expect("Epoch should exist").into();
            }
            sleep(Duration::from_secs(1)).await
        }
    }

    pub async fn init_faucet(&self) -> Result<(), StepError> {
        let client = &self.ctx.namada.client;
        let wallet = self.ctx.namada.wallet.read().await;
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
            if let Ok(is_revealed) = is_pk_revealed(&self.ctx, &faucet_alias, retry_config()).await
            {
                if is_revealed {
                    break;
                }
            }
            if execute_reveal_pk(&self.ctx, faucet_public_key.clone())
                .await
                .is_ok()
            {
                break;
            }
            tracing::warn!("Retry revealing faucet pk...");
            sleep(Duration::from_secs(2)).await;
        }

        Ok(())
    }

    pub async fn try_step(&mut self, next_step: StepType, no_check: bool) -> Code {
        self.step_id += 1;
        tracing::info!("StepID: {}, StepType: {next_step}", self.step_id);

        match self.is_valid(&next_step).await {
            Ok(true) => {}
            _ => {
                tracing::warn!("Invalid step: {next_step} -> {:>?}", self.state);
                let code = Code::Skip(next_step);
                self.stats.update(self.step_id, &code);
                code.output_logs();
                return code;
            }
        }

        tracing::info!("Step is: {next_step}...");
        let tasks = match self.build_tasks(&next_step).await {
            Ok(tasks) if tasks.is_empty() => {
                let code = Code::NoTask(next_step);
                self.stats.update(self.step_id, &code);
                code.output_logs();
                return code;
            }
            Ok(tasks) => tasks,
            Err(e) => {
                let code = Code::StepFailure(next_step, e);
                self.stats.update(self.step_id, &code);
                code.output_logs();
                return code;
            }
        };
        tracing::info!("Built tasks for {next_step}");

        let checks = if no_check {
            vec![]
        } else {
            match self.build_check(&tasks).await {
                Ok(checks) => checks,
                Err(e) => {
                    let code = Code::TaskFailure(next_step, e);
                    self.stats.update(self.step_id, &code);
                    code.output_logs();
                    return code;
                }
            }
        };
        tracing::info!("Built checks for {next_step}");

        let (result, fees) = self.execute(&tasks).await;
        self.apply_fee_payments(&fees);

        let execution_height = match result {
            Ok(height) => height,
            Err(e) => {
                let code = Code::TaskFailure(next_step, e);
                self.stats.update(self.step_id, &code);
                code.output_logs();
                return code;
            }
        };

        tracing::info!("Execution were successful, updating state...");
        if let Err(e) = self.post_execute(&tasks, execution_height).await {
            let code = Code::TaskFailure(next_step, e);
            self.stats.update(self.step_id, &code);
            code.output_logs();
            return code;
        }

        let code = match self.checks(checks, execution_height, &fees).await {
            Ok(_) => Code::Success(next_step),
            Err(e) if matches!(e, CheckError::State(_)) => Code::Fatal(next_step, e),
            Err(e) => Code::CheckFailure(next_step, e),
        };
        self.stats.update(self.step_id, &code);
        code.output_logs();
        code
    }

    pub async fn is_valid(&self, step_type: &StepType) -> Result<bool, StepError> {
        step_type.is_valid(&self.ctx, &self.state).await
    }

    pub async fn build_tasks(&self, step_type: &StepType) -> Result<Vec<Task>, StepError> {
        step_type.build_task(&self.ctx, &self.state).await
    }

    pub async fn build_check(&mut self, tasks: &[Task]) -> Result<Vec<Check>, TaskError> {
        let retry_config = retry_config();
        let mut checks = vec![];
        for task in tasks {
            let built_checks = task.build_checks(&self.ctx, retry_config).await?;
            built_checks.iter().for_each(|check| {
                if let Err(CheckError::PreBalance(details)) = check.check_pre_balance(&self.state) {
                    self.stats
                        .pre_balance_check_failures
                        .entry(self.step_id)
                        .or_default()
                        .insert(check.summary(), details);
                }
            });
            checks.extend(built_checks)
        }
        Ok(checks)
    }

    pub async fn checks(
        &self,
        checks: Vec<Check>,
        execution_height: Height,
        fees: &HashMap<Alias, Fee>,
    ) -> Result<(), CheckError> {
        let retry_config = retry_config();

        if checks.is_empty() {
            return Ok(());
        }

        let check_height = get_block_height(&self.ctx, retry_config)
            .await
            .unwrap_or_default();
        for check in checks {
            tracing::info!("Running {check} check...");
            check
                .do_check(
                    &self.ctx,
                    fees,
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

    pub async fn execute(
        &self,
        tasks: &[Task],
    ) -> (Result<Height, TaskError>, HashMap<Alias, Fee>) {
        let mut fees = HashMap::new();
        let mut execution_height = 0;

        // Execute transactions sequentially.
        // But other workloads could execute transactions at the same block.
        for task in tasks {
            tracing::info!("Executing {task}...");
            let now = Instant::now();
            execution_height = match task.execute(&self.ctx).await {
                Ok(height) => height,
                Err(e) => {
                    match e {
                        // aggreate fees when the tx has been executed
                        TaskError::Execution { .. } | TaskError::IbcTransfer(_) => {
                            task.aggregate_fees(&mut fees, false)
                        }
                        TaskError::InvalidShielded { was_fee_paid, .. } if was_fee_paid => {
                            task.aggregate_fees(&mut fees, false)
                        }
                        _ => {}
                    }
                    return (Err(e), fees);
                }
            };
            tracing::info!("Execution took {}s...", now.elapsed().as_secs());

            task.aggregate_fees(&mut fees, true);
        }

        (Ok(execution_height), fees)
    }

    pub async fn post_execute(
        &mut self,
        tasks: &[Task],
        execution_height: Height,
    ) -> Result<(), TaskError> {
        for task in tasks {
            // update state
            task.update_state(&mut self.state);

            match task {
                Task::ClaimRewards(cr) => {
                    // workaround for exact balance update after claim-rewards
                    let (_, balance) = crate::utils::get_balance(
                        &self.ctx,
                        cr.source(),
                        &Alias::nam().name,
                        crate::utils::retry_config(),
                    )
                    .await?;
                    let balance = balance
                        .to_string()
                        .parse()
                        .expect("Balance conversion shouldn't fail");
                    self.state.overwrite_balance(cr.source(), balance);

                    let claimed_epoch = self.fetch_epoch_at_height(execution_height).await;
                    self.state.set_claimed_epoch(cr.source(), claimed_epoch);
                }
                Task::InitAccount(_) => {
                    // save wallet for init-account
                    let wallet = self.ctx.namada.wallet.read().await;
                    wallet
                        .save()
                        .map_err(|e| TaskError::Wallet(e.to_string()))?;
                }
                Task::DefaultProposal(_) | Task::Vote(_) => {
                    let last_proposal_id = self.state.proposals.keys().max().cloned();
                    let new_proposals = get_proposals(&self.ctx, last_proposal_id).await?;
                    self.state.add_proposals(new_proposals);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn apply_fee_payments(&mut self, fees: &HashMap<Alias, Fee>) {
        fees.iter()
            .for_each(|(payer, fee)| self.state.modify_balance_fee(payer, *fee));
    }
}
