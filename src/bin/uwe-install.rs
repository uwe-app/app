extern crate log;
extern crate pretty_env_logger;

use log::error;
use structopt::StructOpt;

use uwe::Result;

#[derive(Debug, StructOpt)]
#[structopt(name = "uwe-install")]
struct Cli {}

fn fatal(e: &str) {
    error!("{}", e);
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> Result<()> {
    Cli::from_args();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    let name = option_env!("CARGO_PKG_NAME").unwrap().to_string();

    if let Err(e) = release::install(name).await {
        fatal(&e.to_string());
    }

    Ok(())
}
