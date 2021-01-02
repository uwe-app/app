extern crate log;
extern crate pretty_env_logger;

use std::time::SystemTime;

use log::info;
use structopt::StructOpt;

use config::{server::LaunchConfig, ProfileSettings};

use publisher::PublishProvider;

use uwe::{
    self,
    opts::{
        self, fatal, Build, Clean, Dev, Docs, Lang, New, Publish, Server, Sync, Task,
    },
    Error, Result,
};

#[derive(Debug, StructOpt)]
/// Universal web editor
#[structopt(name = "uwe", after_help = "EXAMPLES:
    Start a live reload server: 
        uwe dev .
    Create a release build:
        uwe build .
    Browse offline help:
        uwe docs

Visit https://uwe.app for more guides and information.
    
To upgrade or uninstall use the version manager (uvm).")]
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

    /// Live reload server
    Dev {
        #[structopt(flatten)]
        args: Dev,
    },

    /// Remove the build directory
    Clean {
        #[structopt(flatten)]
        args: Clean,
    },

    /// Create a new project
    New {
        #[structopt(flatten)]
        args: New,
    },

    /// Utility tasks
    Task {
        #[structopt(subcommand)]
        cmd: Task,
    },

    /// Sync project source files
    Sync {
        #[structopt(flatten)]
        args: Sync,
    },

    /// Serve static files
    #[structopt(verbatim_doc_comment)]
    Server {
        #[structopt(flatten)]
        args: Server,
    },

    /// Browse the documentation
    Docs {
        #[structopt(flatten)]
        args: Docs,
    },

    /// Publish a website
    Publish {
        #[structopt(flatten)]
        args: Publish,
    },

    /// Manage translations
    Lang {
        #[structopt(subcommand)]
        cmd: Lang,
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
                remote_name: args.remote_name,
                remote_url: args.remote_url,
            };
            uwe::new::project(opts)?;
        }

        Command::Lang { cmd } => {
            uwe::lang::run(cmd).await?;
        }

        Command::Sync { args } => {
            uwe::sync::run(args).await?;
        }

        Command::Clean { args } => {
            uwe::clean::clean(args.project).await?;
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
            let target = opts::project_path(&args.target)?;

            if !target.exists() || !target.is_dir() {
                return fatal(Error::NotDirectory(target));
            }

            let opts = uwe::opts::server_config(
                &target,
                &args.server,
                config::PORT,
                config::PORT_SSL,
            );

            let launch = LaunchConfig { open: args.open };
            uwe::server::serve(
                &target,
                args.skip_build,
                opts,
                launch,
                args.build_opts,
            )
            .await?;
        }

        Command::Task { cmd } => {
            uwe::task::run(cmd).await?;
        }

        Command::Publish { args } => {
            let project = opts::project_path(&args.project)?;
            let opts = uwe::publish::PublishOptions {
                provider: PublishProvider::Aws,
                env: args.env,
                project,
                exec: args.exec,
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

            //let tls =
                //uwe::opts::tls_config(None, &args.server, config::PORT_SSL);

            let build_args = ProfileSettings {
                paths,
                profile: args.profile,
                //live: Some(args.live),
                //launch: args.launch,
                release: Some(args.release),
                //host: args.server.host,
                //port: args.server.port,
                offline: Some(args.compile.offline),
                exec: Some(args.compile.exec),
                member: args.compile.member,
                include_drafts: Some(args.compile.include_drafts),
                //tls,
                ..Default::default()
            };

            let now = SystemTime::now();
            match uwe::build::compile(&project, build_args).await {
                Ok(_) => {
                    if let Ok(t) = now.elapsed() {
                        info!("{:?}", t);
                    }
                }
                Err(e) => opts::print_error(e),
            }
        }


        Command::Dev { args } => {
            let project = opts::project_path(&args.project)?;

            let paths = if args.paths.len() > 0 {
                Some(args.paths)
            } else {
                None
            };

            let tls =
                uwe::opts::tls_config(None, &args.server, config::PORT_SSL);

            let build_args = ProfileSettings {
                paths,
                profile: args.profile,
                launch: args.launch,
                host: args.server.host,
                port: args.server.port,
                offline: Some(args.compile.offline),
                exec: Some(args.compile.exec),
                member: args.compile.member,
                include_drafts: Some(args.compile.include_drafts),
                tls,
                ..Default::default()
            };

            let now = SystemTime::now();
            match uwe::dev::run(&project, build_args).await {
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
