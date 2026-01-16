use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

pub mod cli;
pub mod error;
pub mod execution;
pub mod socket;

pub fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    if !cli.quiet {
        let filter = match cli.verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        };
        fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive(filter.parse()?))
            .with_writer(std::io::stderr)
            .init();
    }

    tracing::debug!("pfc starting with command: {}", cli.command);

    let ctx = execution::ExecutionContext::new(cli.command, cli.args);
    tracing::debug!("Execution context: {:?}", ctx);

    let socket_path = socket::discover_socket(cli.socket)?;
    tracing::info!("Using socket: {}", socket_path);

    // TODO: Connect to daemon
    // TODO: Transfer context and descriptors

    Ok(())
}
