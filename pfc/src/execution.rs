use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
}

impl ExecutionContext {
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self {
            command,
            args,
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        }
    }
}
