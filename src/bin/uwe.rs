extern crate log;
extern crate pretty_env_logger;

use log::info;
use std::time::SystemTime;
use structopt::StructOpt;

use config::{
    server::{LaunchConfig, TlsConfig},
    ProfileSettings,
};

use publisher::PublishProvider;

use uwe::{
    self,
    opts::{self, fatal, Build, Docs, Publish, Run},
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

        Command::Run { args } => {
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
                profile: args.profile,
                live: Some(args.live),
                release: Some(args.release),
                host: args.server.host,
                port: args.server.port,
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
