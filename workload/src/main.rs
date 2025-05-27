use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use clap::Parser;
use namada_chain_workload::code::Code;
use namada_chain_workload::config::{AppConfig, Args};
use namada_chain_workload::context::Ctx;
use namada_chain_workload::error::CheckError;
use namada_chain_workload::executor::WorkloadExecutor;
use namada_chain_workload::state::State;
use namada_chain_workload::stats::Stats;
use namada_chain_workload::step::StepType;
use tokio::runtime::Builder;
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .without_time()
        .with_ansi(false)
        .init();

    rlimit::increase_nofile_limit(10240).unwrap();
    rlimit::increase_nofile_limit(u64::MAX).unwrap();

    let args = Args::parse();
    let config = match AppConfig::load(args.config) {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Loading the config failed: {e}");
            std::process::exit(4);
        }
    };
    let config = Arc::new(config);
    tracing::info!("Using config: {config:#?}");

    namada_chain_workload::utils::GLOBAL_SEED
        .set(args.seed)
        .expect("Seed already set");
    let end_time = SystemTime::now() + Duration::from_secs(args.test_time_sec);

    let mut handles = Vec::new();
    for _ in 0..args.concurrency {
        let state = State::new();
        let mut stats = Stats::default();

        let ctx = loop {
            match Ctx::new(&config).await {
                Ok(ctx) => break ctx,
                Err(_) => {
                    tracing::info!("Setup Context failed, retrying...");
                    sleep(Duration::from_secs(2)).await;
                }
            }
        };

        let mut executor = WorkloadExecutor::new(ctx, state);
        executor
            .init()
            .await
            .expect("Executor initialization failed");

        let handle = thread::spawn(move || {
            let mut step_id = 0;
            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build Tokio runtime");

            rt.block_on(async move {
                // Initialize accounts
                let code = try_step(
                    &mut executor,
                    StepType::Initialize(Default::default()),
                    false,
                )
                .await;
                if !matches!(code, Code::Success(_)) {
                    stats.update(step_id, &code);
                    return stats;
                }
                try_step(
                    &mut executor,
                    StepType::FundAll(Default::default()),
                    args.no_check,
                )
                .await;
                if !matches!(code, Code::Success(_)) {
                    stats.update(step_id, &code);
                    return stats;
                }

                while end_time > SystemTime::now() {
                    step_id += 1;
                    let next_step = StepType::random_step_type();
                    let code = try_step(&mut executor, next_step, args.no_check).await;

                    stats.update(step_id, &code);
                    code.output_logs();
                }

                stats
            })
        });

        handles.push(handle);
    }

    for h in handles {
        let stats = h.join().expect("No error should happen");
        if !stats.fatal.is_empty() {
            tracing::error!("Fatal failures happened!");
        }
        if !stats.failed.is_empty() {
            tracing::error!("Non-fatal failures happened!");
        }
        if !stats.failure_logs.is_empty() {
            tracing::error!("{:#?}", stats.failure_logs)
        }
    }
}

async fn try_step(executor: &mut WorkloadExecutor, next_step: StepType, no_check: bool) -> Code {
    tracing::info!("StepType: {next_step}");

    match executor.is_valid(&next_step).await {
        Ok(true) => {}
        _ => {
            tracing::warn!("Invalid step: {next_step} -> {:>?}", executor.state());
            return Code::Skip(next_step);
        }
    }

    tracing::info!("Step is: {next_step}...");
    let tasks = match executor.build_tasks(&next_step).await {
        Ok(tasks) if tasks.is_empty() => {
            return Code::NoTask(next_step);
        }
        Ok(tasks) => tasks,
        Err(e) => {
            return Code::StepFailure(next_step, e);
        }
    };
    tracing::info!("Built tasks for {next_step}");

    let checks = if no_check {
        vec![]
    } else {
        match executor.build_check(&tasks).await {
            Ok(checks) => checks,
            Err(e) => return Code::TaskFailure(next_step, e),
        }
    };
    tracing::info!("Built checks for {next_step}");

    let (result, fees) = executor.execute(&tasks).await;
    executor.apply_fee_payments(&fees);

    let execution_height = match result {
        Ok(height) => height,
        Err(e) => return Code::TaskFailure(next_step, e),
    };

    tracing::info!("Execution were successful, updating state...");
    if let Err(e) = executor.post_execute(&tasks, execution_height).await {
        return Code::TaskFailure(next_step, e);
    }

    match executor.checks(checks, execution_height, &fees).await {
        Ok(_) => Code::Success(next_step),
        Err(e) if matches!(e, CheckError::State(_)) => Code::Fatal(next_step, e),
        Err(e) => Code::CheckFailure(next_step, e),
    }
}
