use anyhow::Result;
use context::ExecutionContext;
use discovery::{CreateStrategy, LocalFileStrategy};
use rkyv::{Archived, Deserialize};
use sendfd::RecvWithFd;
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::io::AsRawFd;
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

#[allow(dead_code)]
async fn add_command(ctx: ExecutionContext, mut fds: Vec<OwnedFd>) -> Result<()> {
    if ctx.args.is_empty() {
        if let Some(stderr) = fds.get_mut(2) {
            let mut stderr =
                tokio::fs::File::from_std(unsafe { File::from_raw_fd(stderr.as_raw_fd()) });
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
                if let Some(stderr) = fds.get_mut(2) {
                    let mut stderr =
                        tokio::fs::File::from_std(unsafe { File::from_raw_fd(stderr.as_raw_fd()) });
                    stderr
                        .write_all(format!("error: invalid number '{}'\n", arg).as_bytes())
                        .await?;
                }
                return Err(anyhow::anyhow!("Invalid number: {}", arg));
            }
        }
    }

    if let Some(stdout) = fds.get_mut(1) {
        let mut stdout =
            tokio::fs::File::from_std(unsafe { File::from_raw_fd(stdout.as_raw_fd()) });
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

    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let raw_fd = socket.as_raw_fd();

    loop {
        tokio::select! {
            result = tokio::task::spawn_blocking({
                let mut buf = [0u8; 16384];
                let mut fd_storage = [0; 8];
                move || {
                    use std::os::unix::net::UnixDatagram as StdUnixDatagram;
                    let std_socket = unsafe {
                        StdUnixDatagram::from(OwnedFd::from_raw_fd(raw_fd))
                    };
                    let (n, fds) = std_socket.recv_with_fd(&mut buf, &mut fd_storage)?;
                    let bytes = buf[..n].to_vec();
                    Ok::<_, std::io::Error>((bytes, fds))
                }
            }) => {
                match result {
                    Ok(Ok((bytes, _fds))) => {
                        let archived = unsafe { &*(bytes.as_ptr() as *const Archived<ExecutionContext>) };
                        let context: ExecutionContext = archived.deserialize(&mut rkyv::Infallible)?;
                        tracing::debug!("Deserialized EXECUTION-CONTEXT: command={}, args={:?}",
                            context.command, context.args);
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
