use anyhow::Result;
use clid_context::ExecutionContext;
#[cfg(feature = "local")]
use clid_discovery::{CreateStrategy, LocalFileStrategy};
use rkyv::api::high::from_bytes;
use rkyv::rancor::Error;
use sendfd::RecvWithFd;
use std::collections::HashMap;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::net::UnixDatagram as StdUnixDatagram;
use std::pin::Pin;
use std::sync::Arc;

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

pub async fn worker(child_num: u32, std_socket: StdUnixDatagram, registry: CmdRegistry) {
    let pid = std::process::id();
    tracing::info!("Worker {child_num} started, pid {pid}");

    let std_socket = Arc::new(std::sync::Mutex::new(std_socket));
    std_socket
        .lock()
        .unwrap()
        .set_nonblocking(true)
        .expect("set nonblocking");

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
                                tracing::info!("Worker {child_num}: Received command: {} with {} fds", context.command, fds.len());

                                let command = context.command.clone();
                                let registry = registry.clone();
                                tokio::spawn(async move {
                                    match registry.dispatch(&command, context, fds).await {
                                        Ok(()) => {
                                            tracing::debug!("Worker {child_num}: Command '{}' executed successfully", command);
                                        }
                                        Err(e) => {
                                            tracing::error!("Worker {child_num}: Command '{}' failed: {}", command, e);
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Worker {child_num}: Failed to deserialize: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        } else {
                            tracing::error!("Worker {child_num}: Receive error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Worker {child_num}: Task error: {}", e);
                    }
                }
            }
        }
    }
}

pub fn create_socket() -> Result<String> {
    #[cfg(feature = "local")]
    {
        let strategy = LocalFileStrategy::default();
        let socket_path = strategy.create();
        tracing::info!("Starting daemon on {}", socket_path);
        std::fs::remove_file(&socket_path).ok();
        Ok(socket_path)
    }
    #[cfg(not(feature = "local"))]
    {
        Err(anyhow::anyhow!("No socket creation strategy available (enable a discovery feature)"))
    }
}
