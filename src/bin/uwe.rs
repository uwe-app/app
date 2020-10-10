extern crate pretty_env_logger;
extern crate log;

use log::info;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use structopt::StructOpt;

use config::{
    server::{LaunchConfig, TlsConfig},
    ProfileSettings,
};

use publisher::PublishProvider;

use uwe::{self, Error, Result, opts::{Build, Docs, Init, Publish, Run, Site, fatal, print_error}};

fn get_project_path(input: &PathBuf) -> Result<PathBuf> {
    // NOTE: We want the help output to show "."
    // NOTE: to indicate that the current working
    // NOTE: directory is used but the period creates
    // NOTE: problems with the strip prefix logic for
    // NOTE: live reload so this converts it to the
    // NOTE: empty string.
    let period = Path::new(".").to_path_buf();
    if input == &period {
        return Ok(input.canonicalize()?);
    }
    Ok(input.clone())
}

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
    /// Create a new project
    Init {
        #[structopt(flatten)]
        args: Init,
    },

    /// Compile a site
    Build {
        #[structopt(flatten)]
        args: Build,
    },

    /// Serve static files
    Run {
        #[structopt(flatten)]
        args: Run,
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

    /// Manage sites
    Site {
        #[structopt(flatten)]
        action: Site,
    },
}

impl Command {
    fn default(cli: Cli) -> Self {
        Command::Build {
            args: cli.build_opts,
        }
    }
}

async fn process_command(cmd: Command) -> Result<()> {
    match cmd {
        Command::Init { ref args } => {
            let opts = uwe::init::InitOptions {
                source: args.source.clone(),
                message: args.message.clone(),
                target: args.target.clone(),
                language: args.language.clone(),
                host: args.host.clone(),
                locales: args.locales.clone(),
            };
            uwe::init::init(opts)?;
        }
        Command::Docs { ref args } => {
            let target = uwe::docs::get_target().await?;
            let opts = uwe::opts::server_config(
                &target,
                &args.server,
                config::PORT_DOCS,
                config::PORT_DOCS_SSL,
            );
            uwe::docs::open(opts).await?;
        }
        Command::Run { ref args } => {
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

        Command::Publish { ref args } => {
            let project = get_project_path(&args.project)?;

            let opts = uwe::publish::PublishOptions {
                provider: PublishProvider::Aws,
                env: args.env.clone(),
                project,
            };

            uwe::publish::publish(opts).await?;
        }

        Command::Site { ref action } => match action {
            Site::Add {
                ref name,
                ref project,
            } => {
                let opts = uwe::site::AddOptions {
                    project: project.clone(),
                    name: name.clone(),
                };
                uwe::site::add(opts)?;
            }
            Site::Remove { ref name } => {
                let opts = uwe::site::RemoveOptions {
                    name: name.to_string(),
                };
                uwe::site::remove(opts)?;
            }
            Site::List { .. } => {
                uwe::site::list()?;
            }
        },

        Command::Build { ref args } => {
            let project = get_project_path(&args.project)?;

            let paths = if args.paths.len() > 0 {
                Some(args.paths.clone())
            } else {
                None
            };

            let mut tls: Option<TlsConfig> = None;

            let ssl_port = if let Some(ssl_port) = args.server.ssl_port {
                ssl_port
            } else {
                config::PORT_SSL
            };

            if args.server.ssl_cert.is_some() && args.server.ssl_key.is_some() {
                tls = Some(TlsConfig {
                    cert: args.server.ssl_cert.as_ref().unwrap().to_path_buf(),
                    key: args.server.ssl_key.as_ref().unwrap().to_path_buf(),
                    port: ssl_port,
                });
            }

            let build_args = ProfileSettings {
                paths,
                profile: args.profile.clone(),
                live: Some(args.live),
                release: Some(args.release),
                host: args.server.host.clone(),
                port: args.server.port.clone(),
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
                Err(e) => print_error(e),
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let root_args = Cli::from_args();

    uwe::opts::panic_hook();

    if let Err(e) = uwe::utils::log_level(&*root_args.log_level) {
        return fatal(e);
    }

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

    match root_args.cmd {
        Some(cmd) => {
            if let Err(e) = process_command(cmd).await {
                return fatal(e);
            }
        }
        None => {
            if let Err(e) = process_command(Command::default(root_args)).await {
                return fatal(e);
            }
        }
    }

    Ok(())
}
