use anyhow::Result;
use clap::Parser;
use std::os::unix::net::UnixDatagram as StdUnixDatagram;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "pfd",
    about = "PreFork Daemon - long-running server that receives execution contexts and file descriptors",
    version
)]
pub struct Cli {
    #[arg(short, long, global = true, env = "PFD_LOG")]
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
        let socket_path = pfd::create_socket()?;
        let socket = StdUnixDatagram::bind(&socket_path)?;

        let is_parent = prefork::Prefork::from_resource(socket)
            .with_num_processes(4)
            .with_tokio(pfd::child_runner)
            .fork()
            .expect("fork");

        if is_parent {
            tracing::info!("Parent exit");
            tracing::info!("Cleaning up socket: {}", socket_path);
            std::fs::remove_file(&socket_path)?;
        }

        Ok(())
    }
}
