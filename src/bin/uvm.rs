extern crate log;
extern crate pretty_env_logger;

use structopt::StructOpt;

use uwe::{opts::fatal, Error, Result};

/// Universal (web editor) version manager
#[derive(Debug, StructOpt)]
#[structopt(name = "uvm")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Command,
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

    /// Update the platform tools
    Update {
        /// Update the version manager (uvm)
        #[structopt(short = "s", long = "self")]
        update_self: bool,
    },

    /// Delete a release version
    #[structopt(alias = "rm")]
    Remove { version: String },

    /// Remove old release versions
    Prune {},

    /// Uninstall the toolchain
    Uninstall {},
}

async fn run(cmd: Command, name: &str, version: &str) -> release::Result<()> {
    match cmd {
        Command::Runtime {} => {
            release::fetch().await?;
        }
        Command::List {} => {
            release::list().await?;
        }
        Command::Use { version } => {
            release::select(name, version).await?;
        }
        Command::Install { version } => {
            release::install(name, version).await?;
        }
        Command::Update { update_self } => {
            if update_self {
                release::update_self(version).await?;
            } else {
                release::update(name).await?;
            }
        }
        Command::Remove { version } => {
            release::remove(version).await?;
        }
        Command::Prune {} => {
            release::prune().await?;
        }
        Command::Uninstall {} => {
            release::uninstall().await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    uwe::opts::panic_hook();
    uwe::opts::log_level(&*args.log_level).or_else(fatal)?;

    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");

    run(args.cmd, name, version)
        .await
        .map_err(Error::from)
        .or_else(fatal)?;

    Ok(())
}
