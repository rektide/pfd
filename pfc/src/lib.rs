use anyhow::Result;
use clap::Parser;
use discovery::DiscoveryConfig;

pub mod cli;
pub mod error;
pub mod execution;
mod trace;

pub fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    trace::init(cli.verbose, cli.quiet)?;

    tracing::debug!("pfc starting with command: {}", cli.command);

    let ctx = execution::ExecutionContext::new(cli.command, cli.args);
    tracing::debug!("Execution context: {:?}", ctx);

    let socket_path = discovery::discover_socket(DiscoveryConfig {
        socket_arg: cli.socket,
        ..Default::default()
    })?;
    tracing::info!("Using socket: {}", socket_path);

    // TODO: Connect to daemon
    // TODO: Transfer context and descriptors

    Ok(())
}
