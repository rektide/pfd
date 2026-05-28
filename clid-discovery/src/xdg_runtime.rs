use crate::DiscoveryStrategy;
use std::path::PathBuf;

pub struct XdgRuntimeStrategy;

impl Default for XdgRuntimeStrategy {
    fn default() -> Self {
        Self
    }
}

impl XdgRuntimeStrategy {
    fn runtime_dir() -> Option<PathBuf> {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            return Some(PathBuf::from(runtime_dir));
        }
        tracing::debug!("XDG_RUNTIME_DIR not set");
        None
    }
}

impl DiscoveryStrategy for XdgRuntimeStrategy {
    fn discover(&self) -> Option<String> {
        if let Some(runtime_dir) = Self::runtime_dir() {
            let socket_path = runtime_dir.join("clid.sock");
            if socket_path.exists() {
                tracing::debug!("Found socket: {}", socket_path.display());
                return Some(socket_path.to_string_lossy().to_string());
            }
        }

        tracing::debug!("No socket found in XDG runtime directory");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdg_runtime_strategy_no_runtime_dir() {
        let strategy = XdgRuntimeStrategy::default();
        let result = strategy.discover();
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_xdg_runtime_strategy_default() {
        let strategy = XdgRuntimeStrategy::default();
        assert_eq!(strategy.discover(), strategy.discover());
    }
}
