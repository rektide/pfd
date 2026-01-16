use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "pfd",
    about = "PreFork Daemon - long-running server that receives execution contexts and file descriptors",
    version
)]
pub struct Cli {
    #[arg(short, long, global = true, env = "PFD_LOG")]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    Cli::parse().run().await
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        pfd::run_daemon().await
    }
}
