use anyhow::Result;
use clid_context::ExecutionContext;
use clid_discovery::DiscoveryConfig;
use sendfd::SendWithFd;
use tokio::net::UnixDatagram;

pub async fn send(socket_path: &str, ctx: &ExecutionContext) -> Result<()> {
    let serialized = ctx.serialize()?;
    tracing::debug!("Serialized execution context: {} bytes", serialized.len());

    let socket = UnixDatagram::unbound()?;
    socket.connect(socket_path)?;

    let fds = [0, 1, 2];

    let mut retries = 0;
    loop {
        match socket.send_with_fd(&serialized, &fds) {
            Ok(_) => {
                tracing::info!(
                    "Sent {} bytes with {} file descriptors",
                    serialized.len(),
                    fds.len()
                );
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                retries += 1;
                if retries > 10 {
                    return Err(e.into());
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(10 * retries as u64)).await;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

pub async fn handoff(command: String, args: Vec<String>, socket_arg: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let ctx = ExecutionContext::new(cwd, command).with_args(args);

    let socket_path = clid_discovery::discover_socket(DiscoveryConfig {
        socket_arg,
        ..Default::default()
    })?;
    tracing::info!("Using socket: {}", socket_path);

    send(&socket_path, &ctx).await
}
