use anyhow::Result;
use context::ExecutionContext;
use discovery::{CreateStrategy, LocalFileStrategy};
use rkyv::api::high::from_bytes;
use rkyv::rancor::Error;
use sendfd::RecvWithFd;
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};
use std::os::unix::net::UnixDatagram as StdUnixDatagram;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

type CommandFn = Arc<
    dyn Fn(
            ExecutionContext,
            Vec<OwnedFd>,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

struct CmdRegistryInner {
    commands: HashMap<String, CommandFn>,
}

#[derive(Clone)]
pub struct CmdRegistry {
    inner: Arc<CmdRegistryInner>,
}

impl CmdRegistry {
    pub fn new() -> CmdRegistryBuilder {
        CmdRegistryBuilder {
            commands: HashMap::new(),
        }
    }

    pub async fn dispatch(
        &self,
        command: &str,
        ctx: ExecutionContext,
        fds: Vec<OwnedFd>,
    ) -> Result<()> {
        let handler = self
            .inner
            .commands
            .get(command)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", command))?;
        handler(ctx, fds).await
    }
}

pub struct CmdRegistryBuilder {
    commands: HashMap<String, CommandFn>,
}

impl CmdRegistryBuilder {
    pub fn register<F, Fut>(mut self, name: impl Into<String>, command: F) -> Self
    where
        F: Fn(ExecutionContext, Vec<OwnedFd>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let wrapper = Arc::new(move |ctx: ExecutionContext, fds: Vec<OwnedFd>| {
            Box::pin(command(ctx, fds))
                as Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        });
        self.commands.insert(name.into(), wrapper);
        self
    }

    pub fn build(self) -> CmdRegistry {
        CmdRegistry {
            inner: Arc::new(CmdRegistryInner {
                commands: self.commands,
            }),
        }
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

pub async fn child_runner(child_num: u32, std_socket: StdUnixDatagram) {
    let pid = std::process::id();
    tracing::info!("Child {child_num} started, pid {pid}");

    let registry = CmdRegistry::new()
        .register("add", add_command)
        .build();

    let std_socket = Arc::new(std::sync::Mutex::new(std_socket));
    std_socket.lock().unwrap().set_nonblocking(true).expect("set nonblocking");

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
                        match from_bytes::<ExecutionContext, Error>(&bytes) {
                            Ok(context) => {
                                tracing::info!("Child {child_num}: Received command: {} with {} fds", context.command, fds.len());

                                let command = context.command.clone();
                                let registry = registry.clone();
                                tokio::spawn(async move {
                                    match registry.dispatch(&command, context, fds).await {
                                        Ok(()) => {
                                            tracing::debug!("Child {child_num}: Command '{}' executed successfully", command);
                                        }
                                        Err(e) => {
                                            tracing::error!("Child {child_num}: Command '{}' failed: {}", command, e);
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Child {child_num}: Failed to deserialize: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        } else {
                            tracing::error!("Child {child_num}: Receive error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Child {child_num}: Task error: {}", e);
                    }
                }
            }
        }
    }
}

pub fn create_socket() -> Result<String> {
    let strategy = LocalFileStrategy::default();
    let socket_path = strategy.create();

    tracing::info!("Starting pfd daemon on {}", socket_path);

    std::fs::remove_file(&socket_path).ok();

    tracing::info!("Registered commands: add");

    Ok(socket_path)
}
