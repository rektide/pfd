use anyhow::Result;
use clap::Parser;
use context::ExecutionContext;
use discovery::DiscoveryConfig;
use sendfd::SendWithFd;
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

async fn send_to_daemon(socket_path: &str, data: &[u8]) -> Result<()> {
    let socket = UnixDatagram::unbound()?;
    tracing::debug!("Created unbound socket");

    socket.connect(socket_path)?;
    tracing::debug!("Connected to {}", socket_path);

    let fds = [0, 1, 2];

    let mut retries = 0;
    loop {
        match socket.send_with_fd(data, &fds) {
            Ok(_) => {
                tracing::info!(
                    "Sent {} bytes with {} file descriptors",
                    data.len(),
                    fds.len()
                );
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                retries += 1;
                if retries > 10 {
                    return Err(e.into());
                }
                tracing::debug!("Send would block, retry {}...", retries);
                tokio::time::sleep(tokio::time::Duration::from_millis(10 * retries as u64)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
