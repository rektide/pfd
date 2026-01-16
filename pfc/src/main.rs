#[tokio::main]
async fn main() {
    if let Err(e) = pfc::run().await {
        eprintln!("{}", pfc::error::display_error(&e));
        std::process::exit(1);
    }
}
