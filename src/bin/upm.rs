extern crate log;
extern crate pretty_env_logger;

use std::ffi::OsStr;
use std::path::PathBuf;

use log::info;
use semver::Version;
use structopt::StructOpt;
use url::Url;

use uwe::{
    self,
    opts::{self, fatal},
    plugin::InstallSpec,
    Error, Result,
};

use config::plugin::{ExactPluginSpec, PluginSpec};

fn parse_plugin_spec(src: &str) -> std::result::Result<PluginSpec, Error> {
    src.parse::<PluginSpec>().map_err(Error::from)
}

fn parse_exact_plugin_spec(
    src: &str,
) -> std::result::Result<ExactPluginSpec, Error> {
    src.parse::<ExactPluginSpec>().map_err(Error::from)
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
enum Registry {
    /// Update the local plugin registry
    Update {},

    /// List registry plugins
    #[structopt(alias = "ls")]
    List {
        /// Filter for downloaded archives
        #[structopt(short, long)]
        downloads: bool,

        /// Filter for installed plugins
        #[structopt(short, long)]
        installed: bool,
    },
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Install project dependencies
    #[structopt(alias = "i")]
    Install {
        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },

    /// List project plugin dependencies
    #[structopt(alias = "ls")]
    List {
        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },

    /// Update and list registry packages
    #[structopt(alias = "reg")]
    Registry {
        #[structopt(subcommand)]
        cmd: Registry,
    },

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

    /// Show plugin information
    #[structopt(after_help = "EXAMPLES:
    Print plugin information: 
        upm info std::core
    Print plugin information for a specific version: 
        upm info std::core@4.1.12
")]
    Show {
        #[structopt(parse(try_from_str = parse_exact_plugin_spec))]
        target: ExactPluginSpec,
    },

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

    /// Add a plugin to the installation folder
    #[structopt(after_help = "EXAMPLES:
    Add from the registry: 
        upm add std::core
    Add a specific version from the registry: 
        upm add std::core@4.1.12
    Add from a folder: 
        upm add /path/to/plugin
    Add from an archive: 
        upm add /path/to/plugin/package.tar.xz
    Add from a git repository: 
        upm add https://github.com/username/plugin-repo
")]
    Add {
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
            uwe::plugin::lint(path, inspect).await?;
        }

        Command::Pack { path } => {
            uwe::plugin::pack(path).await?;
        }

        Command::Publish { path } => {
            uwe::plugin::publish(path).await?;
        }

        Command::Clean {} => {
            uwe::plugin::clean().await?;
        }

        Command::Registry { cmd } => match cmd {
            Registry::List {
                downloads,
                installed,
            } => {
                uwe::plugin::list_registry(downloads, installed).await?;
            }

            Registry::Update {} => {
                uwe::plugin::update_registry().await?;
            }
        },

        Command::Show { target } => {
            uwe::plugin::show(target).await?;
        }

        Command::Remove { target } => {
            uwe::plugin::remove(target).await?;
        }

        Command::List { project } => {
            let project = opts::project_path(&project)?;
            uwe::plugin::list_project(project).await?;
        }

        Command::Install { project } => {
            let project = opts::project_path(&project)?;
            uwe::plugin::install(project).await?;
        }

        Command::Add { target, force } => {
            uwe::plugin::add(target, force).await?;
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

    Ok(run(args.cmd).await.map_err(Error::from).or_else(fatal)?)
}
