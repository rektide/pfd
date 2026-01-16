use anyhow::Result;
use discovery::{CreateStrategy, LocalFileStrategy};
use sendfd::RecvWithFd;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::io::AsRawFd;
use tokio::net::UnixDatagram;
use tokio::signal;

pub async fn run_daemon() -> Result<()> {
    let strategy = LocalFileStrategy::default();
    let socket_path = strategy.create();

    tracing::info!("Starting pfd daemon on {}", socket_path);

    std::fs::remove_file(&socket_path).ok();

    let socket = UnixDatagram::bind(&socket_path)?;
    tracing::info!("Listening on {}", socket_path);

    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let raw_fd = socket.as_raw_fd();

    loop {
        tokio::select! {
            result = tokio::task::spawn_blocking({
                let mut buf = [0u8; 1024];
                let mut fd_storage = [0; 8];
                move || {
                    use std::os::unix::net::UnixDatagram as StdUnixDatagram;
                    let std_socket = unsafe {
                        StdUnixDatagram::from(OwnedFd::from_raw_fd(raw_fd))
                    };
                    std_socket.recv_with_fd(&mut buf, &mut fd_storage)
                }
            }) => {
                match result {
                    Ok(Ok((n, fds))) => {
                        tracing::debug!("Received {} bytes, {} fds", n, fds);
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Receive error: {}", e);
                    }
                    Err(e) => {
                        tracing::error!("Task error: {}", e);
                    }
                }
            }
            _ = &mut ctrl_c => {
                tracing::info!("Received shutdown signal");
                break;
            }
        }
    }

    tracing::info!("Cleaning up socket: {}", socket_path);
    std::fs::remove_file(&socket_path)?;

    Ok(())
}
