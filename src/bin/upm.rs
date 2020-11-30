extern crate log;
extern crate pretty_env_logger;

use std::path::PathBuf;
use structopt::StructOpt;

use uwe::{self, opts::fatal, Error, Result};

#[derive(Debug, StructOpt)]
/// Universal (web editor) plugin manager
#[structopt(name = "upm")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Lint a plugin.
    Lint {
        /// Print the computed plugin information.
        #[structopt(short, long)]
        inspect: bool,

        /// Plugin folder.
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Package a plugin.
    Pack {
        /// Plugin folder.
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Publish a plugin.
    #[structopt(alias = "pub")]
    Publish {
        /// Plugin folder.
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Remove all cached plugins.
    Clean {},
}

async fn run(cmd: Command) -> Result<()> {
    match cmd {
        Command::Lint { path, inspect } => {
            uwe::plugin::lint(path, inspect)
                .await
                .map_err(Error::from)?;
        }

        Command::Pack { path } => {
            uwe::plugin::pack(path).await.map_err(Error::from)?;
        }

        Command::Publish { path } => {
            uwe::plugin::publish(path).await.map_err(Error::from)?;
        }

        Command::Clean {} => {
            uwe::plugin::clean().await.map_err(Error::from)?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    uwe::opts::panic_hook();
    uwe::opts::log_level(&*args.log_level).or_else(fatal)?;
    Ok(run(args.cmd).await.or_else(fatal)?)
}
