use crate::CreateStrategy;
use directories::UserDirs;
use std::path::PathBuf;

pub struct XdgRuntimeCreateStrategy {
    socket_name: Option<String>,
}

impl Default for XdgRuntimeCreateStrategy {
    fn default() -> Self {
        Self { socket_name: None }
    }
}

impl XdgRuntimeCreateStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_socket_name(mut self, name: String) -> Self {
        self.socket_name = Some(name);
        self
    }
}

impl CreateStrategy for XdgRuntimeCreateStrategy {
    fn create(&self) -> String {
        let socket_name = self.socket_name.as_deref().unwrap_or("pfd.sock");

        if let Some(user_dirs) = UserDirs::new() {
            if let Some(runtime_dir) = user_dirs.runtime_dir() {
                let socket_path = runtime_dir.join(socket_name);
                return socket_path.to_string_lossy().to_string();
            }
        }

        tracing::warn!("XDG runtime directory not found, falling back to /tmp");
        let fallback = PathBuf::from("/tmp").join(socket_name);
        fallback.to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdg_runtime_create_strategy_default() {
        let strategy = XdgRuntimeCreateStrategy::default();
        let path = strategy.create();
        assert!(path.ends_with("pfd.sock"));
    }

    #[test]
    fn test_xdg_runtime_create_strategy_custom_name() {
        let strategy = XdgRuntimeCreateStrategy::new().with_socket_name("custom.sock".to_string());
        let path = strategy.create();
        assert!(path.ends_with("custom.sock"));
    }
}
