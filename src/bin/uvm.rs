extern crate log;
extern crate pretty_env_logger;

use log::info;
use semver::{Version, VersionReq};
use structopt::StructOpt;

use uwe::{fatal, Error, Result};

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
    /// List release versions
    #[structopt(alias = "ls")]
    List {},

    /// Use a release version
    Use { version: String },

    /// Install a release version
    Install { version: String },

    /// Update to a new release
    Update {
        /// Update the version manager (uvm)
        #[structopt(short = "s", long = "self")]
        update_self: bool,

        /// Semver range filter
        #[structopt(env = "UVM_INSTALL_VERSION_RANGE", hide_env_values = true)]
        version_range: Option<String>,
    },

    /// Delete a release version
    #[structopt(alias = "rm")]
    Remove { version: String },

    /// Remove old release versions
    Prune {},

    /// Uninstall the platform tools
    Uninstall {},
}

async fn run(cmd: Command) -> release::Result<()> {
    let name = config::generator::name();
    let version = config::generator::version();

    match cmd {
        Command::List {} => {
            release::list().await?;
        }
        Command::Use { version } => {
            release::select(name, version).await?;
        }
        Command::Install { version } => {
            release::install(name, version).await?;
        }
        Command::Update {
            update_self,
            version_range,
        } => {
            if update_self {
                release::update_self(version).await?;
            } else {
                let range = if let Some(range) = version_range {
                    Some(range.parse::<VersionReq>()?)
                } else {
                    None
                };
                release::update(name, range).await?;
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
    uwe::panic_hook();
    uwe::log_level(&*args.log_level).or_else(fatal)?;

    // Configure the generator meta data ahead of time

    // Must configure the version here otherwise option_env!() will
    // use the version from the workspace package which we don't really
    // care about, the top-level version is the one that interests us.
    let name = env!("CARGO_PKG_NAME").to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let bin_name = env!("CARGO_BIN_NAME").to_string();
    let user_agent = format!("{}/{}", &name, &version);
    let semver: Version = version.parse().unwrap();

    info!("{}", &version);

    let app_data = config::generator::AppData {
        name,
        bin_name,
        version,
        user_agent,
        semver,
    };
    config::generator::get(Some(app_data));

    run(args.cmd).await.map_err(Error::from).or_else(fatal)?;

    Ok(())
}
