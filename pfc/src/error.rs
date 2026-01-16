pub fn display_error(err: &anyhow::Error) -> String {
    let mut output = String::new();

    output.push_str(&format!("Error: {}\n", err));

    let mut source = err.source();
    while let Some(cause) = source {
        output.push_str(&format!("  Caused by: {}\n", cause));
        source = cause.source();
    }

    if let Some(suggestion) = suggest_fix(err) {
        output.push_str(&format!("\nSuggestion: {}\n", suggestion));
    }

    output
}

fn suggest_fix(err: &anyhow::Error) -> Option<&'static str> {
    let msg = err.to_string().to_lowercase();

    if msg.contains("no pfd socket") {
        Some("Start the pfd daemon first with 'pfd' command")
    } else if msg.contains("permission denied") {
        Some("Check file permissions and try running with appropriate access")
    } else if msg.contains("connection refused") {
        Some("The pfd daemon may not be running. Try starting it first")
    } else {
        None
    }
}
