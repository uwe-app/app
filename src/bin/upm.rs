extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use std::path::PathBuf;
use structopt::StructOpt;
use std::panic;

use uwe::{self, Result, Error};

fn print_error(e: uwe::Error) {
    error!("{}", e);
}

fn fatal(e: uwe::Error) {
    print_error(e);
    std::process::exit(1);
}

#[derive(Debug, StructOpt)]
/// Universal plugin manager
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

async fn process_command(cmd: Plugin) -> Result<()> {
    match cmd {
        Plugin::Lint { path, inspect } => {
            uwe::plugin::lint(path, inspect).await?;
        }
        Plugin::Pack { path } => {
            uwe::plugin::pack(path).await?;
        }
        Plugin::Publish { path } => {
            uwe::plugin::publish(path).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let root_args = Cli::from_args();

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

    if let Err(e) = uwe::utils::log_level(&*root_args.log_level) {
        fatal(e);
    }

    //match &*root_args.log_level {
        //"trace" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        //"debug" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        //"info" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        //"warn" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        //"error" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        //_ => {
            //// Jump a few hoops to pretty print this message
            //let level = &root_args.log_level;
            //env::set_var(LOG_ENV_NAME, "error");
            //pretty_env_logger::init_custom_env(LOG_ENV_NAME);
            //fatal(Error::UnknownLogLevel(level.to_string()));
            //return Ok(());
        //}
    //}

    //pretty_env_logger::init_custom_env(LOG_ENV_NAME);

    if let Err(e) = process_command(root_args.cmd).await {
        fatal(e);
    }

    Ok(())
}
