fn main() {
    let args = std::env::args().collect::<Vec<_>>();

    // Check for --quiet flag early
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");

    if let Err(e) = pfc::run() {
        if !quiet {
            eprintln!("{}", pfc::error::display_error(&e));
        }
        std::process::exit(1);
    }
}
