extern crate log;
extern crate pretty_env_logger;

use std::path::PathBuf;
use std::time::SystemTime;

use log::info;
use structopt::StructOpt;

use config::{
    server::{LaunchConfig, TlsConfig},
    ProfileSettings,
};

use publisher::PublishProvider;

use uwe::{
    self,
    opts::{self, fatal, Alias, Build, Docs, List, New, Publish, Server},
    Error, Result,
};

#[derive(Debug, StructOpt)]
/// Universal web editor
#[structopt(name = "uwe")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Option<Command>,

    #[structopt(flatten)]
    build_opts: Build,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Compile a site
    Build {
        #[structopt(flatten)]
        args: Build,
    },

    /// Create a new project
    New {
        #[structopt(flatten)]
        args: New,
    },

    /// List resources
    #[structopt(alias = "ls")]
    List {
        #[structopt(flatten)]
        args: List,
    },

    /// Serve static files
    #[structopt(alias = "run")]
    Server {
        #[structopt(flatten)]
        args: Server,
    },

    /// Browse the documentation
    Docs {
        #[structopt(flatten)]
        args: Docs,
    },

    /// Publish a site
    Publish {
        #[structopt(flatten)]
        args: Publish,
    },

    Site {
        #[structopt(subcommand)]
        cmd: Site,
    },
}

/// Manage project source files
#[derive(StructOpt, Debug)]
pub enum Site {
    /// Initialize, add files and commit.
    Create {
        #[structopt(short, long)]
        message: String,

        /// Destination path.
        target: PathBuf,
    },

    /// Clone a repository.
    Clone {
        /// Repository URL.
        source: String,

        /// Destination path.
        target: Option<PathBuf>,
    },

    /// Copy a repository (clone and squash)
    Copy {
        /// Initial commit message.
        #[structopt(short, long)]
        message: String,

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

    /// Manage site aliases
    Alias {
        #[structopt(flatten)]
        args: Alias,
    },
}

impl Command {
    fn default(cli: Cli) -> Self {
        Command::Build {
            args: cli.build_opts,
        }
    }
}

async fn run(cmd: Command) -> Result<()> {
    match cmd {
        Command::New { args } => {
            let opts = uwe::new::ProjectOptions {
                source: args.source,
                message: args.message,
                target: args.target,
                language: args.language,
                host: args.host,
                locales: args.locales,
            };
            uwe::new::project(opts)?;
        }

        Command::Site { cmd } => {
            self::site::run(cmd).await?;
        }

        Command::List { args } => {
            if args.blueprints {
                uwe::list::list_blueprints().await?;
            }
        }

        Command::Docs { args } => {
            let target = uwe::docs::get_target().await?;
            let opts = uwe::opts::server_config(
                &target,
                &args.server,
                config::PORT_DOCS,
                config::PORT_DOCS_SSL,
            );
            uwe::docs::open(opts).await?;
        }

        Command::Server { args } => {
            if !args.target.exists() || !args.target.is_dir() {
                return fatal(Error::NotDirectory(args.target.to_path_buf()));
            }

            let opts = uwe::opts::server_config(
                &args.target,
                &args.server,
                config::PORT,
                config::PORT_SSL,
            );

            let launch = LaunchConfig { open: true };

            // Convert to &'static reference
            let opts = server::configure(opts);
            let mut channels = Default::default();

            server::launch(opts, launch, &mut channels).await?;
        }

        Command::Publish { args } => {
            let project = opts::project_path(&args.project)?;
            let opts = uwe::publish::PublishOptions {
                provider: PublishProvider::Aws,
                env: args.env,
                project,
            };
            uwe::publish::publish(opts).await?;
        }

        Command::Build { args } => {
            let project = opts::project_path(&args.project)?;

            let paths = if args.paths.len() > 0 {
                Some(args.paths)
            } else {
                None
            };

            let tls = uwe::opts::tls_config(
                None,
                &args.server,
                config::PORT_SSL,
            );

            let build_args = ProfileSettings {
                paths,
                profile: args.profile,
                live: Some(args.live),
                release: Some(args.release),
                host: args.server.host,
                port: args.server.port,
                offline: Some(args.offline),
                tls,
                ..Default::default()
            };

            let now = SystemTime::now();

            // WARN: Hack for live reload lifetimes!
            // FIXME: use once_cell for the static lifetime!
            let build_args: &'static mut ProfileSettings =
                Box::leak(Box::new(build_args));

            //println!("Compiling with {:?}", &project);

            let error_cb = |e| {
                let _ = fatal(e);
            };

            match uwe::build::compile(&project, build_args, error_cb).await {
                Ok(_) => {
                    if let Ok(t) = now.elapsed() {
                        info!("{:?}", t);
                    }
                }
                Err(e) => opts::print_error(e),
            }
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
    let app_data = config::generator::AppData {
        name,
        version,
        ..Default::default()
    };
    config::generator::get(Some(app_data));

    match args.cmd {
        Some(cmd) => {
            run(cmd).await.or_else(fatal)?;
        }
        None => {
            run(Command::default(args)).await.or_else(fatal)?;
        }
    }

    Ok(())
}

mod site {
    use super::Site;
    use std::path::PathBuf;
    use url::Url;
    use uwe::{opts::Alias, Error, Result};

    fn create(target: PathBuf, message: String) -> Result<()> {
        if !target.exists() || !target.is_dir() {
            return Err(Error::NotDirectory(target.to_path_buf()));
        }

        scm::init(&target, &message)
            .map(|_| ())
            .map_err(Error::from)
    }

    fn clone_or_copy(
        source: String,
        target: Option<PathBuf>,
        pristine: Option<String>,
    ) -> Result<()> {
        let target = if let Some(target) = target {
            target.to_path_buf()
        } else {
            let base = std::env::current_dir()?;

            let mut target_parts =
                source.trim_end_matches("/").split("/").collect::<Vec<_>>();

            let target_name =
                target_parts.pop().ok_or_else(|| Error::NoTargetName)?;
            base.join(target_name)
        };

        let _ = source
            .parse::<Url>()
            .map_err(|_| Error::InvalidRepositoryUrl(source.to_string()))?;

        if target.exists() {
            return Err(Error::TargetExists(target.to_path_buf()));
        }

        if let Some(ref message) = pristine {
            scm::copy(&source, &target, message)
                .map(|_| ())
                .map_err(Error::from)
        } else {
            scm::clone(&source, &target)
                .map(|_| ())
                .map_err(Error::from)
        }
    }

    fn pull(
        target: Option<PathBuf>,
        remote: String,
        branch: String,
    ) -> Result<()> {
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

    pub async fn run(cmd: Site) -> Result<()> {
        match cmd {
            Site::Clone { source, target } => {
                clone_or_copy(source, target, None)?;
            }

            Site::Copy {
                source,
                target,
                message,
            } => {
                clone_or_copy(source, target, Some(message))?;
            }

            Site::Create { target, message } => {
                create(target, message)?;
            }

            Site::Pull {
                target,
                remote,
                branch,
            } => {
                pull(target, remote, branch)?;
            }

            Site::Alias { args } => match args {
                Alias::Add { name, project } => {
                    uwe::alias::add(project, name)?;
                }
                Alias::Remove { name } => {
                    uwe::alias::remove(name)?;
                }
                Alias::List { .. } => {
                    uwe::alias::list()?;
                }
            },
        }

        Ok(())
    }
}
