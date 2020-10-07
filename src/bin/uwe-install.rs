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
    if let Err(e) = release::install().await {
        fatal(&e.to_string());
    }

    Ok(())
}
