use anyhow::Result;
use std::path::Path;

pub async fn discover_socket(cli_socket: Option<String>) -> Result<String> {
    if let Some(socket) = cli_socket {
        return Ok(socket);
    }

    // Local discovery: look for ./pfd.sock or ./.pfd.sock
    let local_sockets = vec!["./pfd.sock", "./.pfd.sock"];

    for socket_path in local_sockets {
        if Path::new(socket_path).exists() {
            tracing::debug!("Found socket: {}", socket_path);
            return Ok(socket_path.to_string());
        }
    }

    // TODO: Add XDG discovery when implemented
    Err(anyhow::anyhow!(
        "No pfd socket found. Try starting pfd daemon first."
    ))
}
