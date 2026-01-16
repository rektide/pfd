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
    pub env_var: Option<String>,
}

impl EnvStrategy {
    pub fn new(env_var: Option<String>) -> Self {
        Self { env_var }
    }

    pub fn with_default(default: &str) -> Self {
        Self {
            env_var: std::env::var(default).ok(),
        }
    }
}

impl DiscoveryStrategy for EnvStrategy {
    fn discover(&self) -> Option<String> {
        match &self.env_var {
            Some(var) => match std::env::var(var) {
                Ok(socket) if !socket.is_empty() => {
                    tracing::debug!("Using socket from {} env: {}", var, socket);
                    Some(socket)
                }
                _ => None,
            },
            None => None,
        }
    }
}

pub struct DiscoveryConfig {
    pub socket_arg: Option<String>,
    pub socket_envs: Option<Vec<String>>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            socket_arg: None,
            socket_envs: Some(vec!["PFC_SOCKET".to_string(), "PFD_SOCKET".to_string()]),
        }
    }
}

pub fn discover_socket(config: DiscoveryConfig) -> anyhow::Result<String> {
    // Priority 1: CLI argument
    if let Some(socket) = config.socket_arg {
        return Ok(socket);
    }

    // Priority 2: Environment variables (try in order)
    if let Some(env_vars) = &config.socket_envs {
        for env_var in env_vars {
            if let Some(socket) = EnvStrategy::new(Some(env_var.clone())).discover() {
                return Ok(socket);
            }
        }
    }

    // Priority 3: Local discovery
    let local_strategy = LocalFileStrategy;
    if let Some(socket) = local_strategy.discover() {
        return Ok(socket);
    }

    Err(anyhow::anyhow!("No pfd socket found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_strategy_with_value() {
        unsafe { std::env::set_var("TEST_SOCKET", "/tmp/test.sock") };
        let strategy = EnvStrategy::new(Some("TEST_SOCKET".to_string()));
        assert_eq!(strategy.discover(), Some("/tmp/test.sock".to_string()));
        unsafe { std::env::remove_var("TEST_SOCKET") };
    }

    #[test]
    fn test_env_strategy_none() {
        unsafe { std::env::remove_var("TEST_SOCKET") };
        let strategy = EnvStrategy::new(None);
        assert_eq!(strategy.discover(), None);
    }

    #[test]
    fn test_env_strategy_empty_string() {
        unsafe { std::env::set_var("TEST_SOCKET", "") };
        let strategy = EnvStrategy::new(Some("TEST_SOCKET".to_string()));
        assert_eq!(strategy.discover(), None);
        unsafe { std::env::remove_var("TEST_SOCKET") };
    }

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(
            config.socket_envs,
            Some(vec!["PFC_SOCKET".to_string(), "PFD_SOCKET".to_string()])
        );
        assert!(config.socket_arg.is_none());
    }

    #[test]
    fn test_discovery_from_pfc_env() {
        unsafe { std::env::set_var("PFC_SOCKET", "/tmp/pfc-from-env.sock") };
        unsafe { std::env::remove_var("PFD_SOCKET") };
        let config = DiscoveryConfig::default();
        let result = discover_socket(config);
        assert_eq!(result.unwrap(), "/tmp/pfc-from-env.sock");
        unsafe { std::env::remove_var("PFC_SOCKET") };
    }

    #[test]
    fn test_discovery_from_pfd_env_fallback() {
        unsafe { std::env::remove_var("PFC_SOCKET") };
        unsafe { std::env::set_var("PFD_SOCKET", "/tmp/pfd.sock") };
        let config = DiscoveryConfig::default();
        let result = discover_socket(config);
        assert_eq!(result.unwrap(), "/tmp/pfd.sock");
        unsafe { std::env::remove_var("PFD_SOCKET") };
    }

    #[test]
    fn test_discovery_priority_pfc_over_pfd() {
        unsafe { std::env::set_var("PFC_SOCKET", "/tmp/pfc.sock") };
        unsafe { std::env::set_var("PFD_SOCKET", "/tmp/pfd2.sock") };
        let config = DiscoveryConfig::default();
        let result = discover_socket(config);
        assert_eq!(result.unwrap(), "/tmp/pfc.sock");
        unsafe { std::env::remove_var("PFC_SOCKET") };
        unsafe { std::env::remove_var("PFD_SOCKET") };
    }
}
