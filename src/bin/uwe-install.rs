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
    //#[structopt(short, long)]
    //runtime: bool,

    /// Uninstall the program
    //#[structopt(long)]
    //remove: bool,

    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Update the runtime assets
    Runtime {},

    /// List release version
    List {},

    /// Upgrade to latest release
    Latest {},

    /// Uninstall the program
    Remove {},
}

fn fatal(e: Error) -> Result<()> {
    error!("{}", e.to_string());
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = Cli::from_args();

    uwe::utils::log_level(&*args.log_level)
        .or_else(fatal)?;

    if let Some(cmd) = args.cmd.take() {
        match cmd {
            Command::Runtime {} => {
                release::runtime().await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
            Command::Remove {} => {
                release::uninstall().await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
            Command::Latest {} => {
                todo!("Intall latest")
            }
            Command::List {} => {
                todo!("List versions")
            }
        } 
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
