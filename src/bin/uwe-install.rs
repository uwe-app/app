extern crate log;
extern crate pretty_env_logger;

use log::error;
use structopt::StructOpt;

use uwe::{Error, Result};

#[derive(Debug, StructOpt)]
#[structopt(name = "uwe-install")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    /// Update the runtime assets
    #[structopt(short, long)]
    runtime: bool,
}

fn fatal(e: Error) -> Result<()> {
    error!("{}", e.to_string());
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> Result<()> {
    let root_args = Cli::from_args();

    uwe::utils::log_level(&*root_args.log_level)
        .or_else(fatal)?;

    if root_args.runtime {
        // Update the runtime assets.
        release::runtime().await
            .map_err(Error::from)
            .or_else(fatal)?;
    } else {
        // Perform a standard installation.
        let name = option_env!("CARGO_PKG_NAME")
            .unwrap().to_string();

        release::install(name).await
            .map_err(Error::from)
            .or_else(fatal)?;
    }


    Ok(())
}
