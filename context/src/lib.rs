use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
pub struct ExecutionContext {
    pub working_dir: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl ExecutionContext {
    pub fn new(working_dir: String, command: String) -> Self {
        Self {
            working_dir,
            command,
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    pub fn with_arg(mut self, arg: String) -> Self {
        self.args.push(arg);
        self
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args.extend(args);
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.push((key, value));
        self
    }

    pub fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        use rkyv::to_bytes;
        to_bytes::<ExecutionContext, 16384>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to serialize execution context: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_creation() {
        let ctx = ExecutionContext::new("/tmp".to_string(), "/bin/ls".to_string());
        assert_eq!(ctx.working_dir, "/tmp");
        assert_eq!(ctx.command, "/bin/ls");
        assert!(ctx.args.is_empty());
    }

    #[test]
    fn test_execution_context_with_args() {
        let ctx = ExecutionContext::new("/tmp".to_string(), "/bin/ls".to_string())
            .with_args(vec!["-la".to_string()]);
        assert_eq!(ctx.args.len(), 1);
        assert_eq!(ctx.args[0], "-la");
    }

    #[test]
    fn test_execution_context_with_env() {
        let ctx = ExecutionContext::new("/tmp".to_string(), "/bin/ls".to_string())
            .with_env("PATH".to_string(), "/usr/bin".to_string());
        assert_eq!(ctx.env.len(), 1);
        assert_eq!(ctx.env[0], ("PATH".to_string(), "/usr/bin".to_string()));
    }

    #[test]
    fn test_serialize_roundtrip() {
        let ctx = ExecutionContext::new("/tmp".to_string(), "/bin/ls".to_string())
            .with_args(vec!["-la".to_string()])
            .with_env("FOO".to_string(), "bar".to_string());

        let bytes = ctx.serialize().unwrap();
        assert!(!bytes.is_empty());
    }
}
