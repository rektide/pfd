use anyhow::Result;
use discovery::{CreateStrategy, LocalFileStrategy};
use sendfd::RecvWithFd;
use tokio::net::UnixListener;
use tokio::signal;

pub async fn run_daemon() -> Result<()> {
    let strategy = LocalFileStrategy::default();
    let socket_path = strategy.create();

    tracing::info!("Starting pfd daemon on {}", socket_path);

    std::fs::remove_file(&socket_path).ok();

    let listener = UnixListener::bind(&socket_path)?;
    tracing::info!("Listening on {}", socket_path);

    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        tracing::debug!("Accepted connection");
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream).await {
                                tracing::error!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
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

async fn handle_connection(stream: tokio::net::UnixStream) -> anyhow::Result<()> {
    tracing::debug!("Handling connection");
    let mut buf = [0u8; 1024];
    let mut fd_storage = [0; 8];
    let (n, fds) =
        tokio::task::spawn_blocking(move || stream.recv_with_fd(&mut buf, &mut fd_storage))
            .await??;
    tracing::debug!("Received {} bytes, {} fds", n, fds);
    Ok(())
}
