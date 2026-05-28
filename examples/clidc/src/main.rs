use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "clidc",
    about = "clid reference client — transfers execution to daemon",
    version
)]
pub struct Cli {
    pub command: String,

    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,

    #[arg(short, long)]
    pub socket: Option<String>,

    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.quiet {
        "off".to_string()
    } else {
        match cli.verbose {
            0 => "warn".to_string(),
            1 => "info".to_string(),
            _ => "debug".to_string(),
        }
    };

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&filter));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    clid_client::handoff(cli.command, cli.args, cli.socket).await
}
