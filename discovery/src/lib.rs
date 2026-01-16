use std::path::Path;

pub trait DiscoveryStrategy {
    fn discover(&self) -> Option<String>;
}

pub trait CreateStrategy {
    fn create(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Hidden,
    NotHidden,
    Both,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Both
    }
}

pub struct LocalFileStrategy {
    pub socket_names: Option<Vec<String>>,
    pub visibility: Option<Visibility>,
}

impl LocalFileStrategy {
    pub fn new() -> Self {
        Self {
            socket_names: None,
            visibility: None,
        }
    }

    pub fn with_socket_names(mut self, names: Vec<String>) -> Self {
        self.socket_names = Some(names);
        self
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = Some(visibility);
        self
    }
}

impl Default for LocalFileStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryStrategy for LocalFileStrategy {
    fn discover(&self) -> Option<String> {
        let socket_names: Vec<&str> = self
            .socket_names
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_else(|| vec!["pfd.sock"]);

        let visibility = self.visibility.unwrap_or_default();

        for name in socket_names {
            if visibility == Visibility::NotHidden || visibility == Visibility::Both {
                let path = format!("./{}", name);
                if Path::new(&path).exists() {
                    tracing::debug!("Found socket: {}", path);
                    return Some(path);
                }
            }
            if visibility == Visibility::Hidden || visibility == Visibility::Both {
                let path = format!("./.{}", name);
                if Path::new(&path).exists() {
                    tracing::debug!("Found socket: {}", path);
                    return Some(path);
                }
            }
        }

        None
    }
}

impl CreateStrategy for LocalFileStrategy {
    fn create(&self) -> String {
        let socket_name = self
            .socket_names
            .as_ref()
            .and_then(|v| v.first())
            .map(|s| s.as_str())
            .unwrap_or("pfd.sock");

        let visibility = self.visibility.unwrap_or_default();

        match visibility {
            Visibility::Hidden => format!("./.{}", socket_name),
            Visibility::NotHidden => format!("./{}", socket_name),
            Visibility::Both => format!("./.{}", socket_name),
        }
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

pub struct DiscoveryConfig {
    pub socket_arg: Option<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self { socket_arg: None }
    }
}

pub fn discover_socket(config: DiscoveryConfig) -> anyhow::Result<String> {
    // Priority 1: CLI argument
    if let Some(socket) = config.socket_arg {
        return Ok(socket);
    }

    // Priority 2: Environment variables
    let env_strategy = EnvStrategy {
        env_var: "PFD_SOCKET".to_string(),
    };
    if let Some(socket) = env_strategy.discover() {
        return Ok(socket);
    }

    // Priority 3: Local discovery
    let local_strategy = LocalFileStrategy::default();
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

    #[test]
    fn test_local_file_strategy_default() {
        let strategy = LocalFileStrategy::default();
        assert_eq!(strategy.socket_names, None);
        assert_eq!(strategy.visibility, None);
    }

    #[test]
    fn test_local_file_strategy_with_names() {
        let strategy = LocalFileStrategy::new()
            .with_socket_names(vec!["test.sock".to_string(), "demo.sock".to_string()]);
        let _expected_paths = ["./test.sock", "./.test.sock", "./demo.sock", "./.demo.sock"];
        assert_eq!(
            strategy.socket_names,
            Some(vec!["test.sock".to_string(), "demo.sock".to_string()])
        );
        assert_eq!(strategy.visibility, None);
    }

    #[test]
    fn test_local_file_strategy_not_hidden() {
        let strategy = LocalFileStrategy::new().with_visibility(Visibility::NotHidden);
        assert_eq!(strategy.visibility, Some(Visibility::NotHidden));
    }

    #[test]
    fn test_local_file_strategy_hidden() {
        let strategy = LocalFileStrategy::new().with_visibility(Visibility::Hidden);
        assert_eq!(strategy.visibility, Some(Visibility::Hidden));
    }

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Both);
    }

    #[test]
    fn test_create_strategy_not_hidden() {
        let strategy = LocalFileStrategy::new()
            .with_socket_names(vec!["test.sock".to_string()])
            .with_visibility(Visibility::NotHidden);
        assert_eq!(strategy.create(), "./test.sock");
    }

    #[test]
    fn test_create_strategy_hidden() {
        let strategy = LocalFileStrategy::new()
            .with_socket_names(vec!["test.sock".to_string()])
            .with_visibility(Visibility::Hidden);
        assert_eq!(strategy.create(), "./.test.sock");
    }

    #[test]
    fn test_create_strategy_both() {
        let strategy = LocalFileStrategy::new()
            .with_socket_names(vec!["test.sock".to_string()])
            .with_visibility(Visibility::Both);
        assert_eq!(strategy.create(), "./.test.sock");
    }

    #[test]
    fn test_create_strategy_default() {
        let strategy = LocalFileStrategy::default();
        assert_eq!(strategy.create(), "./.pfd.sock");
    }
}
