use anyhow::Result;
use context::ExecutionContext;
use discovery::{CreateStrategy, LocalFileStrategy};
use rkyv::Deserialize;
use sendfd::RecvWithFd;
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixDatagram;
use tokio::signal;

type CommandFn = Arc<
    dyn Fn(
            ExecutionContext,
            Vec<OwnedFd>,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

pub struct CmdRegistry {
    commands: HashMap<String, CommandFn>,
}

impl CmdRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register<F, Fut>(&mut self, name: impl Into<String>, command: F)
    where
        F: Fn(ExecutionContext, Vec<OwnedFd>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let wrapper = Arc::new(move |ctx: ExecutionContext, fds: Vec<OwnedFd>| {
            Box::pin(command(ctx, fds))
                as Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        });
        self.commands.insert(name.into(), wrapper);
    }

    pub async fn dispatch(
        &self,
        command: &str,
        ctx: ExecutionContext,
        fds: Vec<OwnedFd>,
    ) -> Result<()> {
        let handler = self
            .commands
            .get(command)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", command))?;
        handler(ctx, fds).await
    }
}

async fn add_command(ctx: ExecutionContext, mut fds: Vec<OwnedFd>) -> Result<()> {
    if ctx.args.is_empty() {
        if fds.len() > 2 {
            let stderr_fd = fds.swap_remove(2);
            let mut stderr =
                tokio::fs::File::from_std(unsafe { File::from_raw_fd(stderr_fd.into_raw_fd()) });
            stderr
                .write_all(b"error: no arguments provided to add command\n")
                .await?;
        }
        return Err(anyhow::anyhow!("No arguments provided"));
    }

    let mut sum: i64 = 0;
    for arg in &ctx.args {
        match arg.parse::<i64>() {
            Ok(n) => sum += n,
            Err(_) => {
                if fds.len() > 2 {
                    let stderr_fd = fds.swap_remove(2);
                    let mut stderr = tokio::fs::File::from_std(unsafe {
                        File::from_raw_fd(stderr_fd.into_raw_fd())
                    });
                    stderr
                        .write_all(format!("error: invalid number '{}'\n", arg).as_bytes())
                        .await?;
                }
                return Err(anyhow::anyhow!("Invalid number: {}", arg));
            }
        }
    }

    if fds.len() > 1 {
        let stdout_fd = fds.swap_remove(1);
        let mut stdout =
            tokio::fs::File::from_std(unsafe { File::from_raw_fd(stdout_fd.into_raw_fd()) });
        stdout.write_all(format!("{}\n", sum).as_bytes()).await?;
    }

    Ok(())
}

pub async fn run_daemon() -> Result<()> {
    let strategy = LocalFileStrategy::default();
    let socket_path = strategy.create();

    tracing::info!("Starting pfd daemon on {}", socket_path);

    std::fs::remove_file(&socket_path).ok();

    let socket = UnixDatagram::bind(&socket_path)?;
    tracing::info!("Listening on {}", socket_path);

    let mut registry = CmdRegistry::new();
    registry.register("add", add_command);
    tracing::info!("Registered commands: add");

    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let std_socket = Arc::new(std::sync::Mutex::new(socket.into_std()?));
    std_socket.lock().unwrap().set_nonblocking(true)?;

    loop {
        tokio::select! {
            result = tokio::task::spawn_blocking({
                let mut buf = [0u8; 16384];
                let mut fd_storage = [0; 8];
                let std_socket = Arc::clone(&std_socket);
                move || {
                    let (n, num_fds) = std_socket.lock().unwrap().recv_with_fd(&mut buf, &mut fd_storage)?;
                    let bytes = buf[..n].to_vec();
                    let fds: Vec<OwnedFd> = fd_storage[..num_fds]
                        .iter()
                        .map(|&fd| unsafe { OwnedFd::from_raw_fd(fd) })
                        .collect();
                    Ok::<_, std::io::Error>((bytes, fds))
                }
            }) => {
                match result {
                    Ok(Ok((bytes, fds))) => {
                        let archived = unsafe { rkyv::archived_root::<ExecutionContext>(&bytes) };
                        let context: ExecutionContext = archived.deserialize(&mut rkyv::Infallible)?;
                        tracing::info!("Received command: {} with {} fds", context.command, fds.len());

                        let command = context.command.clone();
                        match registry.dispatch(&command, context, fds).await {
                            Ok(()) => {
                                tracing::debug!("Command '{}' executed successfully", command);
                            }
                            Err(e) => {
                                tracing::error!("Command '{}' failed: {}", command, e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        } else {
                            tracing::error!("Receive error: {}", e);
                        }
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
