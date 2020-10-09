extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use std::panic;
use std::path::PathBuf;
use structopt::StructOpt;

use uwe::{self, Error, Result};

fn print_error(e: uwe::Error) {
    error!("{}", e);
}

fn fatal(e: uwe::Error) -> Result<()> {
    print_error(e);
    std::process::exit(1);
}

#[derive(Debug, StructOpt)]
/// Universal (web editor) plugin manager
#[structopt(name = "upm")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Plugin,
}

#[derive(StructOpt, Debug)]
enum Plugin {
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();

    // Fluent templates panics if an error is caught parsing the
    // templates (for example attempting to override from a shared resource)
    // so we catch it here and push it out via the log
    panic::set_hook(Box::new(|info| {
        let message = format!("{}", info);
        // NOTE: We must NOT call `fatal` here which explictly exits the program;
        // NOTE: if we did our defer! {} hooks would not get called which means
        // NOTE: lock files would not be removed from disc correctly.
        print_error(Error::Panic(message));
    }));

    uwe::utils::log_level(&*args.log_level).or_else(fatal)?;

    match args.cmd {
        Plugin::Lint { path, inspect } => {
            uwe::plugin::lint(path, inspect)
                .await
                .map_err(Error::from)
                .or_else(fatal)?;
        }
        Plugin::Pack { path } => {
            uwe::plugin::pack(path)
                .await
                .map_err(Error::from)
                .or_else(fatal)?;
        }
        Plugin::Publish { path } => {
            uwe::plugin::publish(path)
                .await
                .map_err(Error::from)
                .or_else(fatal)?;
        }
    }

    Ok(())
}
