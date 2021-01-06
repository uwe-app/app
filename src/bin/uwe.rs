extern crate log;
extern crate pretty_env_logger;

use std::time::SystemTime;

use log::info;
use structopt::StructOpt;
use semver::Version;

use config::{server::LaunchConfig, ProfileSettings, ProfileName};

use publisher::PublishProvider;

use uwe::{
    self,
    opts::{
        self, fatal, Build, Clean, Dev, Docs, Lang, New, Publish, Server, Sync,
        Task,
    },
    Result,
};

#[derive(Debug, StructOpt)]
/// Universal web editor
#[structopt(
    name = "uwe",
    after_help = "EXAMPLES:
    Start a live reload server: 
        uwe dev .
    Preview a release build:
        uwe server . --open
    Create a release build:
        uwe build .
    Browse offline help:
        uwe docs

Visit https://uwe.app for more guides and information.
    
To upgrade or uninstall use the version manager (uvm)."
)]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Compile a site
    ///
    /// Creates a release build of the website into the `build/release` folder; use the `--profile`
    /// option to build to a different location with alternative build settings.
    ///
    /// If the project is a workspace all of the workspace members are compiled; filter the
    /// workspace members to build using the `--member` option.
    Build {
        #[structopt(flatten)]
        args: Build,
    },

    /// Live reload server
    ///
    /// Compiles a debug build of the website into the `build/debug` folder and starts a web
    /// server with live reload enabled watching for changes to the source files in the `site`
    /// folder.
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
            uwe::new::project(opts).await?;
        }

        Command::Lang { cmd } => {
            uwe::lang::run(cmd).await?;
        }

        Command::Sync { args } => {
            uwe::sync::run(args).await?;
        }

        Command::Clean { args } => {
            let project = opts::project_path(&args.project)?;
            uwe::clean::clean(project).await?;
        }

        Command::Docs { args } => {
            let target = uwe::docs::target(args.version_range).await?;
            let opts = uwe::opts::server_config(
                &target,
                &args.server,
                config::PORT_DOCS,
                config::PORT_DOCS_SSL,
            );
            uwe::docs::open(opts).await?;
        }

        Command::Server { args } => {
            let project = opts::project_path(&args.target)?;

            let opts = uwe::opts::server_config(
                &project,
                &args.server,
                config::PORT,
                config::PORT_SSL,
            );

            let launch = LaunchConfig { open: args.open };
            uwe::server::serve(
                &project,
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

            let build_args = ProfileSettings {
                paths,
                release: Some(args.profile.is_none()),
                profile: args.profile.or(Some(ProfileName::Release.to_string())),
                offline: Some(args.compile.offline),
                exec: Some(args.compile.exec),
                member: args.compile.member,
                include_drafts: Some(args.compile.include_drafts),
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
                profile: args.profile.or(Some(ProfileName::Debug.to_string())),
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

            if let Err(e) = uwe::dev::run(&project, build_args).await  {
                opts::print_error(e);
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
