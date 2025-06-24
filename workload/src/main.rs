use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use clap::Parser;
use namada_chain_workload::code::Code;
use namada_chain_workload::config::{AppConfig, Args};
use namada_chain_workload::context::Ctx;
use namada_chain_workload::executor::WorkloadExecutor;
use namada_chain_workload::stats::summary_stats;
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
        .with_thread_ids(true)
        .init();

    rlimit::increase_nofile_limit(10240).unwrap();
    rlimit::increase_nofile_limit(u64::MAX).unwrap();

    let args = Args::parse();
    tracing::info!("Using args: {args:#?}");
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
        let config = Arc::clone(&config);
        let handle = thread::spawn(move || {
            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build Tokio runtime");

            rt.block_on(async move {
                let ctx = loop {
                    match Ctx::new(&config).await {
                        Ok(ctx) => break ctx,
                        Err(e) => {
                            tracing::info!("Setup Context failed: {e}, retrying...");
                            sleep(Duration::from_secs(2)).await;
                        }
                    }
                };
                let mut executor = WorkloadExecutor::new(ctx);
                let thread_id = thread::current().id();

                if args.init {
                    tracing::info!("Initializing accounts for {thread_id:?}...");
                    executor
                        .init_faucet()
                        .await
                        .expect("Executor initialization failed");
                    // Initialize accounts
                    let code = executor
                        .try_step(StepType::Initialize(Default::default()), true)
                        .await;
                    if !matches!(code, Code::Success(_)) {
                        return executor.final_report();
                    }
                    executor
                        .try_step(StepType::FundAll(Default::default()), args.no_check)
                        .await;
                    if !matches!(code, Code::Success(_)) {
                        return executor.final_report();
                    }
                    tracing::info!("Initialization for {thread_id:?} has been completed");
                } else {
                    executor.load_state().expect("Loading state file failed");

                    while end_time > SystemTime::now() {
                        let next_step = StepType::random_step_type();
                        executor.reconnect(&config);
                        executor.try_step(next_step, args.no_check).await;
                    }
                }

                executor.save_state().expect("Saving state failed");
                let stats = executor.final_report();
                println!("{stats}");
                stats
            })
        });

        handles.push(handle);
    }

    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("No error should happen"))
        .collect();
    let is_successful = summary_stats(results, !args.init);

    std::process::exit(if is_successful { 0 } else { 1 });
}
