extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use log::info;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use structopt::StructOpt;

use std::panic;

use config::{
    server::{HostConfig, LaunchConfig, ServerConfig, TlsConfig},
    ProfileSettings,
};
use publisher::PublishProvider;

use uwe::{self, Error, Result};

fn get_server_config(
    target: &PathBuf,
    opts: &WebServerOpts,
    default_port: u16,
    default_port_ssl: u16,
) -> ServerConfig {
    let serve: ServerConfig = Default::default();
    let mut host = &serve.listen;
    let mut port = &default_port;
    let mut tls = serve.tls.clone();

    let ssl_port = if let Some(ssl_port) = opts.ssl_port {
        ssl_port
    } else {
        default_port_ssl
    };

    if let Some(ref h) = opts.host {
        host = h;
    }
    if let Some(ref p) = opts.port {
        port = p;
    }

    if opts.ssl_cert.is_some() && opts.ssl_key.is_some() {
        tls = Some(TlsConfig {
            cert: opts.ssl_cert.as_ref().unwrap().to_path_buf(),
            key: opts.ssl_key.as_ref().unwrap().to_path_buf(),
            port: ssl_port,
        });
    }

    let host = HostConfig::new(target.clone(), host.to_owned(), None, None);

    ServerConfig::new_host(host, port.to_owned(), tls)
}

fn compiler_error(e: &compiler::Error) {
    match e {
        compiler::Error::Multi { ref errs } => {
            error!("Compile error ({})", errs.len());
            for e in errs {
                error!("{}", e);
            }
            std::process::exit(1);
        }
        _ => {}
    }

    error!("{}", e);
}

fn print_error(e: uwe::Error) {
    match e {
        uwe::Error::Compiler(ref e) => {
            return compiler_error(e);
        }
        uwe::Error::Workspace(ref e) => match e {
            workspace::Error::Compiler(ref e) => {
                return compiler_error(e);
            }
            _ => {}
        },
        _ => {}
    }
    error!("{}", e);
}

fn fatal(e: uwe::Error) {
    print_error(e);
    std::process::exit(1);
}

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
    build_opts: BuildOpts,
}

#[derive(StructOpt, Debug)]
struct BuildOpts {
    /// Build profile name
    #[structopt(long)]
    profile: Option<String>,

    /// Enable live reload
    #[structopt(short, long)]
    live: bool,

    /// Generate a release build
    #[structopt(short, long)]
    release: bool,

    #[structopt(flatten)]
    server: WebServerOpts,

    /// Read config from directory
    #[structopt(parse(from_os_str), default_value = ".")]
    project: PathBuf,

    /// Compile only these paths
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
struct InitOpts {
    /// Commit message.
    #[structopt(short, long)]
    message: Option<String>,

    /// Language for the new project
    #[structopt(short, long)]
    language: Option<String>,

    /// Host name for the new project
    #[structopt(short, long)]
    host: Option<String>,

    /// Set multiple languages (comma delimited)
    #[structopt(short = "L", long)]
    locales: Option<String>,

    /// Output directory for the new project
    #[structopt(parse(from_os_str))]
    target: PathBuf,

    /// Repository URL, folder or blueprint name.
    #[structopt()]
    source: Option<String>,
}

#[derive(StructOpt, Debug)]
struct PublishOpts {
    /// Publish environment
    #[structopt()]
    env: String,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    project: PathBuf,
}

#[derive(StructOpt, Debug)]
struct RunOpts {
    #[structopt(flatten)]
    server: WebServerOpts,

    /// Directory to serve files from
    #[structopt(parse(from_os_str))]
    target: PathBuf,
}

#[derive(StructOpt, Debug)]
struct WebServerOpts {
    /// The name of the host
    #[structopt(short, long)]
    host: Option<String>,

    /// The port number
    #[structopt(short, long)]
    port: Option<u16>,

    /// The port number for SSL
    #[structopt(long)]
    ssl_port: Option<u16>,

    /// Path to an SSL certificate file
    #[structopt(long, env, hide_env_values = true)]
    ssl_cert: Option<PathBuf>,

    /// Path to an SSL key file
    #[structopt(long, env, hide_env_values = true)]
    ssl_key: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
struct DocsOpts {
    #[structopt(flatten)]
    server: WebServerOpts,
}

#[derive(StructOpt, Debug)]
enum Site {
    /// Add a site
    Add {
        /// Project folder
        #[structopt(parse(from_os_str))]
        project: PathBuf,

        /// Project name
        name: Option<String>,
    },
    /// Remove a site
    #[structopt(alias = "rm")]
    Remove {
        /// The project name
        name: String,
    },
    /// List sites
    #[structopt(alias = "ls")]
    List {},
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Create a new project
    Init {
        #[structopt(flatten)]
        args: InitOpts,
    },

    /// Compile a site
    Build {
        #[structopt(flatten)]
        args: BuildOpts,
    },

    /// Serve static files
    Run {
        #[structopt(flatten)]
        args: RunOpts,
    },

    /// Browse the documentation
    Docs {
        #[structopt(flatten)]
        args: DocsOpts,
    },

    /// Publish a site
    Publish {
        #[structopt(flatten)]
        args: PublishOpts,
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
            let opts = get_server_config(
                &target,
                &args.server,
                config::PORT_DOCS,
                config::PORT_DOCS_SSL,
            );
            uwe::docs::open(opts).await?;
        }
        Command::Run { ref args } => {
            if !args.target.exists() || !args.target.is_dir() {
                fatal(Error::NotDirectory(args.target.to_path_buf()));
                return Ok(());
            }

            let opts = get_server_config(
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

            println!("Compiling with {:?}", &project);

            match uwe::build::compile(&project, build_args, fatal).await {
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
                fatal(e);
            }
        }
        None => {
            if let Err(e) = process_command(Command::default(root_args)).await {
                fatal(e);
            }
        }
    }

    Ok(())
}
