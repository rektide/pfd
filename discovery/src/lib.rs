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

pub struct EnvStrategy {
    pub env_var: String,
}

impl DiscoveryStrategy for EnvStrategy {
    fn discover(&self) -> Option<String> {
        match std::env::var(&self.env_var) {
            Ok(socket) if !socket.is_empty() => {
                tracing::debug!("Using socket from {} env: {}", self.env_var, socket);
                Some(socket)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_strategy_with_value() {
        unsafe { std::env::set_var("TEST_SOCKET", "/tmp/test.sock") };
        let strategy = EnvStrategy {
            env_var: "TEST_SOCKET".to_string(),
        };
        assert_eq!(strategy.discover(), Some("/tmp/test.sock".to_string()));
        unsafe { std::env::remove_var("TEST_SOCKET") };
    }

    #[test]
    fn test_env_strategy_empty() {
        unsafe { std::env::set_var("TEST_SOCKET", "") };
        let strategy = EnvStrategy {
            env_var: "TEST_SOCKET".to_string(),
        };
        assert_eq!(strategy.discover(), None);
        unsafe { std::env::remove_var("TEST_SOCKET") };
    }

    #[test]
    fn test_env_strategy_not_set() {
        unsafe { std::env::remove_var("TEST_SOCKET") };
        let strategy = EnvStrategy {
            env_var: "TEST_SOCKET".to_string(),
        };
        assert_eq!(strategy.discover(), None);
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
    let env_strategy = EnvStrategy {
        env_var: config.socket_env.clone(),
    };
    if let Some(socket) = env_strategy.discover() {
        return Ok(socket);
    }

    // Priority 3: Local discovery
    let local_strategy = LocalFileStrategy;
    if let Some(socket) = local_strategy.discover() {
        return Ok(socket);
    }

    Err(anyhow::anyhow!("No pfd socket found"))
}
