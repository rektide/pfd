use anyhow::Result;
use clap::Parser;

pub mod cli;
pub mod execution;
pub mod socket;

pub async fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    tracing::debug!("pfc starting with command: {}", cli.command);

    // TODO: Discover socket
    // TODO: Serialize execution context
    // TODO: Connect to daemon
    // TODO: Transfer context and descriptors

    Ok(())
}
