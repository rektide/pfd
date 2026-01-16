use anyhow::Result;
use clap::Parser;

pub mod cli;
pub mod execution;
pub mod socket;

pub async fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    tracing::debug!("pfc starting with command: {}", cli.command);

    let ctx = execution::ExecutionContext::new(cli.command, cli.args);
    tracing::debug!("Execution context: {:?}", ctx);

    let socket_path = socket::discover_socket(cli.socket).await?;
    tracing::info!("Using socket: {}", socket_path);

    // TODO: Connect to daemon
    // TODO: Transfer context and descriptors

    Ok(())
}
