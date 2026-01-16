use clap::Parser;

#[derive(Parser)]
#[command(
    name = "pfc",
    about = "Minimal prefork client for execution transfer",
    version,
    long_about = "Transfers execution context to pfd daemon and terminates",
    color = clap::ColorChoice::Auto,
)]
pub struct Cli {
    /// Command to execute (e.g., echo, ls, cat)
    pub command: String,

    /// Arguments to pass to the command
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,

    /// Path to pfd socket (default: ./pfd.sock)
    #[arg(short, long)]
    pub socket: Option<String>,

    /// Increase logging verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}
