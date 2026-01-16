use std::path::Path;

pub trait DiscoveryStrategy {
    fn discover(&self) -> Option<String>;
}

pub struct LocalFileStrategy;

impl DiscoveryStrategy for LocalFileStrategy {
    fn discover(&self) -> Option<String> {
        let local_sockets = vec!["./pfd.sock", "./.pfd.sock"];

        for socket_path in local_sockets {
            if Path::new(socket_path).exists() {
                tracing::debug!("Found socket: {}", socket_path);
                return Some(socket_path.to_string());
            }
        }

        None
    }
}

pub struct DiscoveryConfig {
    pub socket_arg: Option<String>,
    pub socket_env: String,
}

pub fn discover_socket(config: DiscoveryConfig) -> anyhow::Result<String> {
    // Priority 1: CLI argument
    if let Some(socket) = config.socket_arg {
        return Ok(socket);
    }

    // Priority 2: Environment variable
    if let Ok(socket) = std::env::var(&config.socket_env) {
        tracing::debug!("Using socket from {} env: {}", config.socket_env, socket);
        return Ok(socket);
    }

    // Priority 3: Local discovery
    let local_strategy = LocalFileStrategy;
    if let Some(socket) = local_strategy.discover() {
        return Ok(socket);
    }

    Err(anyhow::anyhow!("No pfd socket found"))
}
