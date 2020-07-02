extern crate pretty_env_logger;

#[macro_use]
extern crate log;

use log::info;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use structopt::StructOpt;

use std::panic;

use utils;

use hypertext::{
    BuildArguments, Config, Error,
};

use hypertext::command;

use publisher::PublishProvider;

const LOG_ENV_NAME: &'static str = "HYPERTEXT_LOG";

fn fatal(e: impl std::error::Error) {
    error!("{}", e);
    std::process::exit(1);
}

fn error(s: String) {
    fatal(Error::new(s));
}

fn get_project_path(input: PathBuf) -> PathBuf {

    // NOTE: We want the help output to show "."
    // NOTE: to indicate that the current working
    // NOTE: directory is used but the period creates
    // NOTE: problems with the strip prefix logic for
    // NOTE: live reload so this converts it to the
    // NOTE: empty string.
    let period = Path::new(".").to_path_buf();
    let empty = Path::new("").to_path_buf();
    let mut project = input.clone();
    if project == period {
        project = empty;
    }
    project
}

#[derive(Debug, StructOpt)]
/// Fast and elegant site generator
#[structopt(name = "hypertext")]
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
enum Book {
    /// Add a book
    Add {
        /// Book path relative to project
        #[structopt(parse(from_os_str))]
        path: PathBuf,

        /// Project folder
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },
    /// List books
    #[structopt(alias="ls")]
    List {
        /// Project folder
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },
    /// Build books
    Build {
        /// Project folder
        #[structopt(parse(from_os_str))]
        project: PathBuf,

        /// Target book to build
        #[structopt(parse(from_os_str))]
        target: Vec<PathBuf>,
    },
}

#[derive(StructOpt, Debug)]
struct BuildOpts {
    /// Build tag name
    #[structopt(short, long)]
    tag: Option<String>,

    /// Set the default layout file
    #[structopt(long)]
    layout: Option<PathBuf>,

    /// Maximum depth to traverse
    #[structopt(short, long)]
    max_depth: Option<usize>,

    /// Enable live reload
    #[structopt(short, long)]
    live: bool,

    /// Disable incremental build
    #[structopt(long)]
    force: bool,

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

    #[structopt(subcommand)]
    action: Option<InitCommands>,

    /// Private key to use for SSH connections
    #[structopt(short, long, env = "HT_SSH_PRIVATE_KEY", hide_env_values = true)]
    private_key: Option<PathBuf>,

    /// Target directory for the project
    #[structopt(parse(from_os_str))]
    // Not that normally we want a path but when --list
    // is given clap will error without the Option
    target: Option<PathBuf>,

    /// The blueprint source path or URL
    #[structopt()]
    source: Option<String>,
}

#[derive(StructOpt, Debug)]
enum InitCommands {
    /// List available blueprints
    #[structopt(alias="ls")]
    List {},
}

#[derive(StructOpt, Debug)]
struct FetchOpts {
    /// Update the blueprint cache
    #[structopt(short, long)]
    blueprint: bool,

    /// Update the standalone cache
    #[structopt(short, long)]
    standalone: bool,

    /// Update the documentation cache
    #[structopt(short, long)]
    documentation: bool,

    /// Update the release cache
    #[structopt(short, long)]
    release: bool,
}

#[derive(StructOpt, Debug)]
struct UpgradeOpts {}

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
}

#[derive(StructOpt, Debug)]
struct DocsOpts {}

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
    #[structopt(alias="rm")]
    Remove {
        /// The project name
        name: String,
    },
    /// List sites
    #[structopt(alias="ls")]
    List {},
}

#[derive(StructOpt, Debug)]
enum Command {

    /// Create, list and build books
    Book {
        #[structopt(flatten)]
        action: Book,
    },

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

    /// Serve site files
    Run {
        #[structopt(flatten)]
        args: RunOpts,
    },

    /// Browse the documentation
    Docs {
        #[structopt(flatten)]
        args: DocsOpts,
    },

    /// Update cached repositories
    Fetch {
        #[structopt(flatten)]
        args: FetchOpts,
    },

    /// Upgrade to latest
    Upgrade {
        #[structopt(flatten)]
        args: UpgradeOpts,
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

fn process_command(cmd: &Command) {
    match cmd {
        Command::Book{ ref action } => {
            match action {
                Book::Add { ref path, ref project } => {
                    // TODO
                },
                Book::List { ref project } => {
                    let opts = command::book::BookOptions{
                        project: project.clone(),
                        ..Default::default()
                    };
                    if let Err(e) = command::book::list(opts) {
                        fatal(e);
                    }
                },
                Book::Build{ ref project, ref target } => {
                    let opts = command::book::BookOptions{
                        project: project.clone(),
                        target: target.clone(),
                    };
                    if let Err(e) = command::book::build(opts) {
                        fatal(e);
                    }
                },
            }
        },

        Command::Init { ref args } => {
            let opts = command::blueprint::InitOptions {
                source: args.source.clone(),
                target: args.target.clone(),
                private_key: args.private_key.clone(),
            };

            if let Some(ref action) = args.action {
                match action {
                    InitCommands::List {} => {
                        if let Err(e) = command::blueprint::list() {
                            fatal(e);
                        }
                    },
                }
            } else {
                if let Err(e) = command::blueprint::init(opts) {
                    fatal(e);
                }
            }

        }
        Command::Fetch { ref args } => {
            let opts = command::fetch::FetchOptions {
                blueprint: args.blueprint,
                standalone: args.standalone,
                documentation: args.documentation,
                release: args.release,
            };

            if let Err(e) = command::fetch::update(opts) {
                fatal(e);
            }
        }
        Command::Upgrade { .. } => {
            if let Err(e) = command::upgrade::try_upgrade() {
                fatal(e);
            }
        }

        Command::Docs { .. } => {
            if let Err(e) = command::docs::open() {
                fatal(e);
            }
        }

        Command::Run { ref args } => {
            let cfg: Config = Default::default();
            let serve = cfg.serve.as_ref().unwrap();
            let mut host = &serve.host;
            let mut port = &serve.port;

            if let Some(h) = &args.server.host {
                host = h;
            }

            if let Some(p) = &args.server.port {
                port = p;
            }

            let opts = command::run::ServeOptions {
                target: args.target.clone(),
                host: host.to_owned(),
                port: port.to_owned(),
                open_browser: true,
                watch: None,
                endpoint: utils::generate_id(16),
                redirects: None,
            };

            if !opts.target.exists() || !opts.target.is_dir() {
                error(format!(
                    "Not a directory '{}'",
                    opts.target.display()
                ));
            }

            if let Err(e) = command::run::serve_only(opts) {
                fatal(e);
            }
        }

        Command::Publish { ref args } => {
            let project = get_project_path(args.project.clone());

            let opts = command::publish::PublishOptions {
                provider: PublishProvider::Aws,
                env: args.env.clone(),
                project,
            };

            if let Err(e) = command::publish::publish(opts) {
                fatal(e);
            }
        }

        Command::Site { ref action } => {
            match action {
                Site::Add { ref name, ref project } => {
                    let opts = command::site::AddOptions {
                        project: project.clone(),
                        name: name.clone(),
                    };
                    if let Err(e) = command::site::add(opts) {
                        fatal(e);
                    }
                },
                Site::Remove { ref name } => {
                    let opts = command::site::RemoveOptions {
                        name: name.to_string(),
                    };
                    if let Err(e) = command::site::remove(opts) {
                        fatal(e);
                    }
                },
                Site::List { .. } => {
                    let opts = command::site::ListOptions{};
                    if let Err(e) = command::site::list(opts) {
                        fatal(e);
                    }
                },
            }
        },

        Command::Build { ref args } => {
            let project = get_project_path(args.project.clone());

            let paths = if args.paths.len() > 0 {
                Some(args.paths.clone())
            } else {
                None
            };

            let build_args = BuildArguments {
                paths,
                tag: args.tag.clone(),
                max_depth: args.max_depth,
                live: Some(args.live),
                force: Some(args.force),
                release: Some(args.release),
                host: args.server.host.clone(),
                port: args.server.port.clone(),
                layout: args.layout.clone(),
                ..Default::default()
            };

            let now = SystemTime::now();
            //if let Err(e) = hypertext::build(cfg, opts, fatal) {
            if let Err(e) = command::build::compile(&project, &build_args, fatal) {
                fatal(e);
            }
            if let Ok(t) = now.elapsed() {
                info!("{:?}", t);
            }
        }
    }
}

fn main() {
    let root_args = Cli::from_args();

    // Fluent templates panics if an error is caught parsing the
    // templates (for example attempting to override from a shared resource)
    // so we catch it here and push it out via the log
    panic::set_hook(Box::new(|info| {
        let message = format!("{}", info);
        fatal(Error::new(message));
    }));

    match &*root_args.log_level {
        "trace" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        "debug" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        "info" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        "warn" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        "error" => env::set_var(LOG_ENV_NAME, &root_args.log_level),
        _ => {
            // Jump a few hoops to pretty print this message
            let level = &root_args.log_level;
            env::set_var(LOG_ENV_NAME, "error");
            pretty_env_logger::init_custom_env(LOG_ENV_NAME);
            error(format!("unknown log level: {}", level));
        }
    }

    pretty_env_logger::init_custom_env(LOG_ENV_NAME);

    match &root_args.cmd {
        Some(cmd) => {
            process_command(cmd);
        }
        None => {
            process_command(&Command::default(root_args));
        }
    }
}
