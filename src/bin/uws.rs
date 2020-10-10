extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use std::panic;
use std::path::PathBuf;
use std::ffi::OsStr;
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

fn parse_os_url(s: &OsStr) -> Url {
    let u: Url = s.to_string_lossy().parse().unwrap();
    u
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
        #[structopt(parse(from_os_str = parse_os_url))]
        source: Url,

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
            //uwe::plugin::lint(path, inspect)
                //.await
                //.map_err(Error::from)
                //.or_else(fatal)?;
        }
    }

    Ok(())
}
