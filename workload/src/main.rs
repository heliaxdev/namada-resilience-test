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
            let code = Code::ConfigFatal(e.to_string());
            code.output_logs();
            std::process::exit(code.code());
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
            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build Tokio runtime");

            rt.block_on(async move {
                // Initialize accounts
                try_step(
                    &mut executor,
                    StepType::Initialize(Default::default()),
                    false,
                )
                .await;
                try_step(
                    &mut executor,
                    StepType::FundAll(Default::default()),
                    args.no_check,
                )
                .await;

                while end_time > SystemTime::now() {
                    let next_step = StepType::random_step_type();
                    try_step(&mut executor, next_step, args.no_check).await;
                }
            });
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
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

    let exit_code = match executor.checks(checks, execution_height, &fees).await {
        Ok(_) => Code::Success(next_step),
        Err(e) if matches!(e, CheckError::State(_)) => Code::Fatal(next_step, e),
        Err(e) => Code::CheckFailure(next_step, e),
    };

    tracing::info!("Statistics: {:>?}", executor.state().stats);

    exit_code
}
