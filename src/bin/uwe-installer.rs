#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use log::error;
use structopt::StructOpt;

use updater;

#[derive(Debug, StructOpt)]
#[structopt(name = "uwe-installer", version = "1.0.0")]
struct Cli {}

fn fatal(e: &str) {
    error!("{}", e);
    std::process::exit(1);
}

fn main() {
    Cli::from_args();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    if let Err(e) = updater::install() {
        fatal(&e.to_string());
    }
}
