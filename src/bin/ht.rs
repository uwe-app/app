extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use log::info;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use structopt::StructOpt;

use std::panic;

use hypertext::{
    BuildArguments, BundleOptions, Config, DocsOptions, Error, InitOptions, PrefOptions,
    ServeOptions, UpdateOptions, UpgradeOptions,
};

const LOG_ENV_NAME: &'static str = "HYPER_LOG";

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
struct BuildOpts {
    /// Build tag name
    #[structopt(short, long)]
    tag: Option<String>,

    /// Build input sub-directory
    #[structopt(short, long)]
    directory: Option<PathBuf>,

    /// Set the default layout file
    #[structopt(long)]
    layout: Option<PathBuf>,

    /// Maximum depth to traverse
    #[structopt(short, long)]
    max_depth: Option<usize>,

    /// Use index.html for directory links
    #[structopt(long)]
    include_index: bool,

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

    /// Just process these paths
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
}

#[derive(StructOpt, Debug)]
struct InitOpts {
    /// List available blueprints
    #[structopt(short, long)]
    list: bool,

    /// Private key to use for SSH connections
    #[structopt(short, long)]
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
struct UpdateOpts {
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
struct PrefOpts {
    /// Edit the preferences file
    #[structopt(short, long)]
    edit: bool,
}

#[derive(StructOpt, Debug)]
struct ServeOpts {
    #[structopt(flatten)]
    server: WebServerOpts,

    /// Target directory to serve files from
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
struct BundleOpts {
    /// Force overwrite generated files
    #[structopt(long)]
    force: bool,

    /// Keep intermediary source files
    #[structopt(short, long)]
    keep: bool,

    /// Bundle for Linux
    #[structopt(short, long)]
    linux: bool,

    /// Bundle for MacOs
    #[structopt(short, long)]
    mac: bool,

    /// Bundle for Windows
    #[structopt(short, long)]
    windows: bool,

    /// The name of the generated bundle
    #[structopt(short, long)]
    name: Option<String>,

    /// Directory containing website files to bundle
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Generate bundle executables in directory
    #[structopt(parse(from_os_str), default_value = "build")]
    output: PathBuf,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Create a new project from a blueprint
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
    Serve {
        #[structopt(flatten)]
        args: ServeOpts,
    },

    /// Browse the documentation
    Docs {
        #[structopt(flatten)]
        args: DocsOpts,
    },

    /// Bundle a site into executables (requires Go)
    Bundle {
        #[structopt(flatten)]
        args: BundleOpts,
    },

    /// Manage preferences
    Pref {
        #[structopt(flatten)]
        args: PrefOpts,
    },

    /// Update cached repositories
    Update {
        #[structopt(flatten)]
        args: UpdateOpts,
    },

    /// Upgrade to latest hypertext
    Upgrade {
        #[structopt(flatten)]
        args: UpgradeOpts,
    },
}

impl Command {
    fn default(cli: Cli) -> Self {
        Command::Build {
            args: cli.build_opts,
        }
    }
}

fn fatal(e: impl std::error::Error) {
    error!("{}", e);
    std::process::exit(1);
}

fn error(s: String) {
    fatal(Error::new(s));
}

fn create_output_dir(output: &PathBuf) {
    if !output.exists() {
        info!("mkdir {}", output.display());
        if let Err(e) = fs::create_dir_all(output) {
            fatal(e);
        }
    }

    if !output.is_dir() {
        error(format!("Not a directory: {}", output.display()));
    }
}

fn process_command(cmd: &Command) {
    match cmd {
        Command::Init { ref args } => {
            let opts = InitOptions {
                source: args.source.clone(),
                target: args.target.clone(),
                list: args.list,
                private_key: args.private_key.clone(),
            };

            if let Err(e) = hypertext::init(opts) {
                fatal(e);
            }
        }
        Command::Update { ref args } => {
            let opts = UpdateOptions {
                blueprint: args.blueprint,
                standalone: args.standalone,
                documentation: args.documentation,
                release: args.release,
            };

            if let Err(e) = hypertext::update(opts) {
                fatal(e);
            }
        }
        Command::Upgrade { .. } => {
            let opts = UpgradeOptions {};
            if let Err(e) = hypertext::upgrade(opts) {
                fatal(e);
            }
        }
        Command::Pref { ref args } => {
            let opts = PrefOptions { edit: args.edit };

            if let Err(e) = hypertext::pref(opts) {
                fatal(e);
            }
        }

        Command::Docs { .. } => {
            let opts = DocsOptions {};
            if let Err(e) = hypertext::docs(opts) {
                fatal(e);
            }
        }

        Command::Serve { ref args } => {
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

            let opts = ServeOptions {
                target: args.target.clone(),
                host: host.to_owned(),
                port: port.to_owned(),
                open_browser: true,
                watch: None,
                endpoint: hypertext::generate_id(16),
            };

            if !opts.target.exists() || !opts.target.is_dir() {
                error(format!(
                    "Directory does not exist: {}",
                    opts.target.display()
                ));
            }

            if let Err(e) = hypertext::serve_only(opts) {
                fatal(e);
            }
        }

        Command::Bundle { ref args } => {
            if !args.input.exists() || !args.input.is_dir() {
                error(format!(
                    "Directory does not exist: {}",
                    args.input.display()
                ));
            }

            create_output_dir(&args.output);

            let opts = BundleOptions {
                source: args.input.clone(),
                target: args.output.clone(),
                force: args.force,
                keep: args.keep,
                linux: args.linux,
                mac: args.mac,
                windows: args.windows,
                name: args.name.clone(),
            };

            if let Err(e) = hypertext::bundle(opts) {
                fatal(e);
            }
        }

        Command::Build { ref args } => {
            // NOTE: We want the help output to show "."
            // NOTE: to indicate that the current working
            // NOTE: directory is used but the period creates
            // NOTE: problems with the strip prefix logic for
            // NOTE: live reload so this converts it to the
            // NOTE: empty string.
            let period = Path::new(".").to_path_buf();
            let empty = Path::new("").to_path_buf();
            let mut project = args.project.clone();
            if project == period {
                project = empty;
            }

            let paths = if args.paths.len() > 0 {
                Some(args.paths.clone())
            } else {
                None
            };

            let build_args = BuildArguments {
                paths,
                directory: args.directory.clone(),
                tag: args.tag.clone(),
                max_depth: args.max_depth,
                include_index: Some(args.include_index),
                live: Some(args.live),
                force: Some(args.force),
                release: Some(args.release),
                host: args.server.host.clone(),
                port: args.server.port.clone(),
                layout: args.layout.clone(),
                base: None,
            };

            let now = SystemTime::now();
            //if let Err(e) = hypertext::build(cfg, opts, fatal) {
            if let Err(e) = hypertext::build_project(&project, &build_args, fatal) {
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
