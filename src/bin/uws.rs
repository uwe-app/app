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

        /// Destination path.
        target: Option<PathBuf>,
    },

    /// Pull a repository.
    Pull {
        #[structopt(short, long, default_value = "origin")]
        remote: String,

        #[structopt(short, long, default_value = "master")]
        branch: String,

        /// Repository path.
        target: Option<PathBuf>,
    },
}

fn clone(source: String, target: Option<PathBuf>) -> Result<()> {
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
        .map_err(|_| Error::InvalidRepositoryUrl(source.to_string()))?;

    if target.exists() {
        return Err(
            Error::TargetExists(target.to_path_buf()));
    }

    scm::clone(&source, &target)
        .map(|_| ())
        .map_err(Error::from)
}

fn pull(target: Option<PathBuf>, remote: String, branch: String) -> Result<()> {
    let target = if let Some(target) = target {
        target.to_path_buf()
    } else {
        std::env::current_dir()?
    };

    if !target.exists() || !target.is_dir() {
        return Err(Error::NotDirectory(target.to_path_buf()));
    }

    scm::open(&target)
        .map_err(|_| Error::NotRepository(target.to_path_buf()))?;

    scm::pull(&target, Some(remote), Some(branch))
        .map(|_| ())
        .map_err(Error::from)
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
        Command::Clone { source, target } => {
            clone(source, target).or_else(fatal)
        }
        Command::Pull { target, remote, branch } => {
            pull(target, remote, branch).or_else(fatal)
        }
    }
}
