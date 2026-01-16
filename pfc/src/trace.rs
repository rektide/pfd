use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init(verbose: u8, quiet: bool) -> Result<()> {
    if quiet {
        return Ok(());
    }

    let filter = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter));

    let subscriber = fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("Failed to set tracing subscriber: {}", e))?;

    Ok(())
}
