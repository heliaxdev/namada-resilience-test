#[derive(clap::Parser, Clone)]
pub struct AppConfig {
    #[clap(long, env)]
    #[arg(required = true)]
    pub rpc: String,
}
