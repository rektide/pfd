use anyhow::Result;
use clap::Parser;
use context::ExecutionContext;
use discovery::DiscoveryConfig;
use sendfd::SendWithFd;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::io::AsRawFd;
use tokio::net::UnixDatagram;

pub mod cli;
pub mod error;
mod trace;

pub async fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    trace::init(cli.verbose, cli.quiet)?;

    tracing::debug!("pfc starting with command: {}", cli.command);

    let ctx = ExecutionContext::new(
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string()),
        cli.command,
    )
    .with_args(cli.args.clone());

    let serialized = ctx.serialize()?;
    tracing::debug!("Serialized execution context: {} bytes", serialized.len());

    let socket_path = discovery::discover_socket(DiscoveryConfig {
        socket_arg: cli.socket,
        ..Default::default()
    })?;
    tracing::info!("Using socket: {}", socket_path);

    send_to_daemon(&socket_path, &serialized).await?;

    Ok(())
}

async fn send_to_daemon(_socket_path: &str, data: &[u8]) -> Result<()> {
    let socket = UnixDatagram::unbound()?;

    let stdin_fd = unsafe { OwnedFd::from_raw_fd(0) };
    let stdout_fd = unsafe { OwnedFd::from_raw_fd(1) };
    let stderr_fd = unsafe { OwnedFd::from_raw_fd(2) };

    let fds = [
        stdin_fd.as_raw_fd(),
        stdout_fd.as_raw_fd(),
        stderr_fd.as_raw_fd(),
    ];

    socket.send_with_fd(data, &fds)?;

    tracing::info!(
        "Sent {} bytes with {} file descriptors",
        data.len(),
        fds.len()
    );

    Ok(())
}
