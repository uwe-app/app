extern crate pretty_env_logger;
extern crate log;

use std::path::PathBuf;

use structopt::StructOpt;
use url::Url;

use uwe::{self, Error, Result, opts::{fatal}};

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

    /// Initialize, add files and commit.
    Create {
        #[structopt(short, long)]
        message: String,

        /// Destination path.
        target: PathBuf,
    },

    /// Clone a repository.
    Clone {
        /// Remove history and replace with this commit message.
        #[structopt(short, long)]
        pristine: Option<String>,

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

fn create(target: PathBuf, message: String) -> Result<()> {
    if !target.exists() || !target.is_dir() {
        return Err(Error::NotDirectory(target.to_path_buf()));
    }

    scm::init(&target, &message)
        .map(|_| ())
        .map_err(Error::from)
}

fn clone(source: String, target: Option<PathBuf>, pristine: Option<String>) -> Result<()> {
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

    scm::clone(&source, &target, None)
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

    uwe::opts::panic_hook();

    uwe::utils::log_level(&*args.log_level).or_else(fatal)?;

    match args.cmd {
        Command::Create { target, message } => {
            create(target, message).or_else(fatal)
        }
        Command::Clone { source, target, pristine } => {
            clone(source, target, pristine).or_else(fatal)
        }
        Command::Pull { target, remote, branch } => {
            pull(target, remote, branch).or_else(fatal)
        }
    }
}
