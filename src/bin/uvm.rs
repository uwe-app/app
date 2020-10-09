extern crate log;
extern crate pretty_env_logger;

use log::error;
use structopt::StructOpt;

use uwe::{Error, Result};

/// Universal (web editor) version manager
#[derive(Debug, StructOpt)]
#[structopt(name = "uvm")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Update the runtime assets
    Runtime {},

    /// List release versions
    #[structopt(alias = "ls")]
    List {},

    /// Use a release version
    Use { version: String },

    /// Install a release version
    Install { version: String },

    /// Upgrade to latest release
    Latest {},

    /// Update the version manager (uvm)
    Update {},

    /// Delete a release version
    #[structopt(alias = "rm")]
    Remove { version: String },

    /// Remove old release versions
    Prune {},

    /// Uninstall the program
    Uninstall {},
}

fn fatal(e: Error) -> Result<()> {
    error!("{}", e.to_string());
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = Cli::from_args();

    uwe::utils::log_level(&*args.log_level).or_else(fatal)?;

    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");

    if let Some(cmd) = args.cmd.take() {
        match cmd {
            Command::Runtime {} => {
                release::fetch().await.map_err(Error::from).or_else(fatal)?;
            }
            Command::List {} => {
                release::list().await.map_err(Error::from).or_else(fatal)?;
            }
            Command::Use { version } => {
                release::select(name, version)
                    .await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
            Command::Install { version } => {
                release::install(name, version)
                    .await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
            Command::Latest {} => {
                release::latest(name)
                    .await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
            Command::Update {} => {
                release::update(version)
                    .await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
            Command::Remove { version } => {
                release::remove(version)
                    .await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
            Command::Prune {} => {
                release::prune().await.map_err(Error::from).or_else(fatal)?;
            }
            Command::Uninstall {} => {
                release::uninstall()
                    .await
                    .map_err(Error::from)
                    .or_else(fatal)?;
            }
        }
    } else {
        // Perform a standard installation.
        release::latest(name)
            .await
            .map_err(Error::from)
            .or_else(fatal)?;
    }

    Ok(())
}
