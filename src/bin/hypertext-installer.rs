extern crate pretty_env_logger;
extern crate log;

use log::{info, error};
//use std::time::SystemTime;
use structopt::StructOpt;

use hypertext::updater;

#[derive(Debug, StructOpt)]
#[structopt(name = "hypertext-installer", version = "1.0.0")]
struct Cli {}

fn fatal(e: &str) {
    error!("{}", e);
    std::process::exit(1);
}

fn main() {
    Cli::from_args();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    match updater::update() {
        Ok(bin) => {
            info!("Installed {}", bin.display());
        },
        Err(e) => fatal(&e.to_string()),
    }
}
