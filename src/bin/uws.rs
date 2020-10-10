extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use std::panic;
use std::path::PathBuf;

use structopt::StructOpt;
use url::Url;

use uwe::{self, Error, Result};

fn print_error(e: uwe::Error) {
    error!("{}", e);
}

fn fatal(e: uwe::Error) -> Result<()> {
    print_error(e);
    std::process::exit(1);
}

#[derive(Debug, StructOpt)]
/// Universal (web editor) sync
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
    /// Clone a repository.
    Clone {
        /// Repository URL.
        source: String,

        /// Destination folder.
        target: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();

    panic::set_hook(Box::new(|info| {
        let message = format!("{}", info);
        print_error(Error::Panic(message));
    }));

    uwe::utils::log_level(&*args.log_level).or_else(fatal)?;

    match args.cmd {
        Command::Clone { source, target} => {

            let target = if let Some(target) = target {
                target.to_path_buf()
            } else {
                let base = std::env::current_dir()?;

                let mut target_parts = source
                    .trim_end_matches("/")
                    .split("/").collect::<Vec<_>>();

                let target_name = target_parts.pop().ok_or_else(
                    || Error::NoTargetName)?;
                base.join(target_name)
            };

            let _ = source.parse::<Url>()
                .map_err(|_| fatal(Error::InvalidRepositoryUrl(source.to_string())));

            if target.exists() {
                return fatal(
                    Error::TargetExists(target.to_path_buf()));
            }

            scm::clone(&source, &target)
                .map(|_| ())
                .map_err(Error::from)
                .or_else(fatal)?;
        }
    }

    Ok(())
}
