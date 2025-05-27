use std::path::PathBuf;

use serde::Deserialize;

use crate::error::Error;

#[derive(clap::Parser, Clone, Debug)]
pub struct Args {
    #[clap(long, env)]
    #[arg(required = true)]
    pub config: PathBuf,
    #[clap(long, env)]
    #[clap(default_value_t = false)]
    pub no_check: bool,
    #[clap(long, env)]
    #[arg(required = true)]
    pub seed: u64,
    #[clap(long, env)]
    #[arg(required = true)]
    pub concurrency: u64,
    #[clap(long, env)]
    #[arg(required = true)]
    pub test_time_sec: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub chain_id: String,
    pub rpc: String,
    pub masp_indexer_url: String,
    pub faucet_sk: String,
    pub cosmos_rpc: String,
    pub cosmos_grpc: String,
    pub cosmos_base_dir: PathBuf,
    pub namada_channel_id: String,
    pub cosmos_channel_id: String,
}

impl AppConfig {
    pub fn load(path: PathBuf) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path).map_err(|e| Error::Config(e.to_string()))?;
        toml::from_str(&content).map_err(|e| Error::Config(e.to_string()))
    }
}
