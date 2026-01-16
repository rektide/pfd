use clap::CommandFactory;
use clap_complete::{generate_to, shells::*};
use std::env;

include!("pfc/src/cli.rs");

fn main() -> std::io::Result<()> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };

    let mut cmd = pfc::cli::Cli::command();
    let name = cmd.get_name().to_string();

    generate_to(Bash, &mut cmd, &name, &outdir)?;
    generate_to(Zsh, &mut cmd, &name, &outdir)?;
    generate_to(Fish, &mut cmd, &name, &outdir)?;

    println!("cargo:rerun-if-changed=pfc/src/cli.rs");
    Ok(())
}
