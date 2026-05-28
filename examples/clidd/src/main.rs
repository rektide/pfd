use anyhow::Result;
use clid_daemon::{create_socket, worker, CmdRegistry};
use clap::Parser;
use std::os::unix::net::UnixDatagram as StdUnixDatagram;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "clidd", about = "clid reference daemon", version)]
pub struct Cli {
    #[arg(short, long, global = true, env = "CLIDD_LOG")]
    verbose: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    Cli::parse().run()
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let socket_path = create_socket()?;
        let socket = StdUnixDatagram::bind(&socket_path)?;

        tracing::info!("Registered commands: add");

        let registry = CmdRegistry::new()
            .register("add", |_ctx, _fds| async { Ok(()) })
            .build();

        #[cfg(feature = "prefork")]
        {
            let is_parent = prefork::Prefork::from_resource(socket)
                .with_num_processes(4)
                .with_tokio(move |child_num, std_socket| {
                    let registry = registry.clone();
                    async move { worker(child_num, std_socket, registry).await }
                })
                .fork()
                .expect("fork");

            if is_parent {
                tracing::info!("Parent exit");
                tracing::info!("Cleaning up socket: {}", socket_path);
                std::fs::remove_file(&socket_path)?;
            }
        }

        #[cfg(not(feature = "prefork"))]
        {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                worker(0, socket, registry).await;
            });
        }

        Ok(())
    }
}
