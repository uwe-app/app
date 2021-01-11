extern crate log;
extern crate pretty_env_logger;

use std::ffi::OsStr;
use std::path::PathBuf;

use log::info;
use semver::Version;
use structopt::StructOpt;
use url::Url;

use uwe::{self, opts::fatal, plugin::InstallSpec, Error, Result};

use config::plugin::{ExactPluginSpec, PluginSpec};

fn parse_plugin_spec(src: &str) -> std::result::Result<PluginSpec, Error> {
    src.parse::<PluginSpec>().map_err(Error::from)
}

fn parse_install_spec(src: &str) -> std::result::Result<InstallSpec, Error> {
    // Treat as a git url
    let repo_url: Option<Url> = if let Ok(url) = src.parse::<Url>() {
        if url.has_authority() {
            Some(url)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(url) = repo_url {
        Ok(InstallSpec::Repo(url))
    } else {
        let path = PathBuf::from(src);
        let spec: Option<InstallSpec> = if path.exists() && path.is_dir() {
            Some(InstallSpec::Folder(path))
        } else {
            if path.exists() && path.is_file() {
                if let Some(name) = path.file_name() {
                    let archive_name = OsStr::new(config::PACKAGE_NAME);
                    if name == archive_name {
                        Some(InstallSpec::Archive(path))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(spec) = spec {
            Ok(spec)
        } else {
            let plugin_spec = src.parse::<ExactPluginSpec>()?;
            Ok(InstallSpec::Plugin(plugin_spec))
        }
    }
}

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
    /// Update the local plugin registry
    Update {},

    /// Lint a plugin
    Lint {
        /// Print the computed plugin information
        #[structopt(short, long)]
        inspect: bool,

        /// Plugin folder
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    /// Package a plugin
    Pack {
        /// Plugin folder
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },

    /// Publish a plugin
    #[structopt(alias = "pub")]
    Publish {
        /// Plugin folder.
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },

    /// Delete all installed plugins
    Clean {},

    /// Remove installed plugin(s)
    #[structopt(
        alias = "rm",
        after_help = "EXAMPLES:
    Remove all versions of a plugin: 
        upm rm std::core
    Remove a specific version: 
        upm rm std::core@=4.1.12
    Remove all versions with major version 4: 
        upm rm std::core@^4
"
    )]
    Remove {
        #[structopt(parse(try_from_str = parse_plugin_spec))]
        target: PluginSpec,
    },

    /// Install a plugin
    #[structopt(alias = "i")]
    Install {
        /// Force overwrite existing installed plugin
        #[structopt(short, long)]
        force: bool,

        #[structopt(parse(try_from_str = parse_install_spec))]
        target: InstallSpec,
    },
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

        Command::Update {} => {
            uwe::plugin::update().await.map_err(Error::from)?;
        }

        Command::Remove { target } => {
            uwe::plugin::remove(target).await.map_err(Error::from)?;
        }

        Command::Install { target, force } => {
            uwe::plugin::install(target, force).await.map_err(Error::from)?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    uwe::opts::panic_hook();
    uwe::opts::log_level(&*args.log_level).or_else(fatal)?;

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

    Ok(run(args.cmd).await.or_else(fatal)?)
}
