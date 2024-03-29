extern crate log;
extern crate pretty_env_logger;

use std::path::PathBuf;

use log::info;
use semver::Version;
use structopt::StructOpt;

use uwe::{self, fatal, opts, Error, Result};

use config::{
    href::UrlPath,
    plugin::{ExactPluginSpec, PluginSpec},
};

/// Universal (web editor) plugin manager
#[derive(Debug, StructOpt)]
#[structopt(name = "upm")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Clean {
    /// Remove all downloads and plugins
    All,

    /// Remove installed plugins (archive)
    Archives,

    /// Remove download cache
    Downloads,

    /// Remove installed plugins
    Plugins,

    /// Remove installed plugins (git)
    Repositories,
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

    /// Remove cached files
    Clean {
        /// Show matched files but do not delete them
        #[structopt(short, long)]
        dry_run: bool,

        #[structopt(subcommand)]
        cmd: Clean,
    },

    /// List project dependencies
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

    /// Show plugin information
    #[structopt(after_help = "EXAMPLES:
    Print plugin information:
        upm info std::core
    Print plugin information for a specific version:
        upm info std::core@4.1.12
")]
    Show { target: ExactPluginSpec },

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
    Remove { target: PluginSpec },

    /// Add a plugin
    ///
    /// The target plugin will be installed if it does not exist; if the
    /// plugin already exists use the --force option to overwrite it.
    ///
    /// Options --path, --archive, --git and <plugin-name>
    /// are mutually exclusive; it is an error to combine them.
    #[structopt(after_help = "EXAMPLES:
    Add from the registry:
        upm add std::core
    Add a specific version from the registry:
        upm add std::core@4.1.12
    Add from a folder:
        upm add --path /path/to/plugin
    Add from an archive:
        upm add --archive /path/to/plugin/package.tar.xz
    Add from a git repository:
        upm add --git https://github.com/username/plugin-repo
")]
    Add {
        /// Force overwrite existing plugin
        #[structopt(short, long)]
        force: bool,

        /// Path to a plugin folder.
        #[structopt(short, long, parse(from_os_str))]
        path: Option<PathBuf>,

        /// Path to a plugin archive.
        #[structopt(short, long, parse(from_os_str))]
        archive: Option<PathBuf>,

        /// URL for a git repository.
        #[structopt(short, long)]
        git: Option<String>,

        /// Folder prefix for git repositories.
        #[structopt(long)]
        prefix: Option<UrlPath>,

        /// Plugin name.
        plugin_name: Option<ExactPluginSpec>,
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

        Command::Clean { dry_run, cmd } => match cmd {
            Clean::All => uwe::plugin::clean::all(dry_run).await?,
            Clean::Downloads => uwe::plugin::clean::downloads(dry_run).await?,
            Clean::Archives => uwe::plugin::clean::archives(dry_run).await?,
            Clean::Repositories => {
                uwe::plugin::clean::repositories(dry_run).await?
            }
            Clean::Plugins => uwe::plugin::clean::plugins(dry_run).await?,
        },

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

        Command::Add {
            plugin_name,
            path,
            archive,
            git,
            prefix,
            force,
        } => {
            uwe::plugin::add(plugin_name, path, archive, git, prefix, force)
                .await?;
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

    Ok(run(args.cmd).await.map_err(Error::from).or_else(fatal)?)
}
