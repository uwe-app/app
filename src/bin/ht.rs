extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use log::info;
use std::env;
use std::fs;
use std::time::SystemTime;
use std::path::PathBuf;
use structopt::StructOpt;

use hypertext::{
    BuildTag,
    Error,
    BuildOptions,
    ServeOptions,
    InitOptions,
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

#[derive(StructOpt,Debug)]
struct BuildOpts {
    /// Build tag name
    #[structopt(short, long)]
    tag: Option<String>,

    /// Follow symbolic links
    #[structopt(short, long)]
    follow_links: bool,

    /// Build input sub-directory
    #[structopt(short, long)]
    directory: Option<PathBuf>,

    /// Maximum depth for recursion
    #[structopt(long)]
    max_depth: Option<usize>,

    /// Enable live reload
    #[structopt(short, long)]
    live: bool,

    /// Generate a release build
    #[structopt(short, long)]
    release: bool,

    /// Disable strict mode
    #[structopt(long)]
    loose: bool,

    /// Read files from directory
    #[structopt(parse(from_os_str), default_value = "site")]
    input: PathBuf,

    /// Write files to directory
    #[structopt(parse(from_os_str), default_value = "build")]
    output: PathBuf,
}

#[derive(StructOpt,Debug)]
struct InitOpts {

    /// The name of a template to use
    #[structopt(short, long, default_value = "newcss")]
    template: String,

    /// Target directory to create
    #[structopt(parse(from_os_str))]
    target: PathBuf,
}

#[derive(StructOpt,Debug)]
struct ServeOpts {
    /// The name of the host
    #[structopt(short, long, default_value = "localhost")]
    host: String,

    /// The port number
    #[structopt(short, long, default_value = "8989")]
    port: String,

    /// Target directory to serve files from
    #[structopt(parse(from_os_str))]
    target: PathBuf,
}

#[derive(StructOpt,Debug)]
enum Command {
    /// Create a site
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
    }
}

impl Command {
    fn default(cli: Cli) -> Self {
        Command::Build {
            args: cli.build_opts
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

fn process_command(cmd: &Command) {
    match cmd {
        Command::Init {
            ref args
        } => {
            let opts = InitOptions {
                target: args.target.clone(),
                template: args.template.clone(),
            };

            if opts.target.exists() {
                error(format!("directory already exists: {}", opts.target.display()));
            }

            if let Err(e) = hypertext::init(opts) {
                fatal(e);
            }
        },
        Command::Serve {
            ref args
        } => {
            let opts = ServeOptions {
                target: args.target.clone(),
                host: args.host.clone(),
                port: args.port.clone(),
                open_browser: true,
                watch: None,
            };

            if !opts.target.exists() {
                error(format!("directory does not exist: {}", opts.target.display()));
            }

            if let Err(e) = hypertext::serve(opts, |_, _| { Ok(()) }) {
                fatal(e);
            }
        },
        Command::Build {ref args} => {

            if !args.input.is_dir() {
                error(format!("not a directory: {}", args.input.display()));
            }

            if !args.output.exists() {
                if let Err(e) = fs::create_dir(&args.output) {
                    fatal(e);
                }
            }

            if !args.output.is_dir() {
                error(format!("not a directory: {}", args.output.display()));
            }

            let mut tag_target = BuildTag::Debug;
            if args.release {
                tag_target = BuildTag::Release;
            }

            if let Some(t) = &args.tag {
                if !t.is_empty() {
                    tag_target = BuildTag::Custom(t.to_string());
                }
            }

            let target_dir = tag_target.get_path_name();
            info!("{}", target_dir);

            let mut target = args.output.clone();

            if !target_dir.is_empty() {
                let mut target_dir_buf = PathBuf::new();
                target_dir_buf.push(&target_dir);

                if target_dir_buf.is_absolute() {
                    error(format!("build tag may not be an absolute path {}", target_dir));
                }

                target.push(target_dir);
            }

            let mut dir = None;
            if let Some(d) = &args.directory {
                if d.is_absolute() {
                    error(format!("directory must be relative {}", d.display()));
                }
                let mut src = args.input.clone();
                src.push(d);
                if !src.exists() {
                    error(format!("target directory does not exist {}", src.display()));
                }
                dir = Some(src);
            }

            let opts = BuildOptions {
                source: args.input.clone(),
                target,
                output: args.output.clone(),
                directory: dir,
                max_depth: args.max_depth,
                follow_links: args.follow_links,
                clean_url: true,
                strict: !args.loose,
                release: args.release,
                live: args.live,
                livereload: None,
                tag: tag_target,
            };

            let now = SystemTime::now();
            if let Err(e) = hypertext::build(opts) {
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
        },
        None => {
            process_command(&Command::default(root_args));
        }
    }
}
