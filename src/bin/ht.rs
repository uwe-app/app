extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use log::info;
use std::env;
use std::fs;
use std::time::SystemTime;
use std::path::PathBuf;
use structopt::StructOpt;

use hypertext::{build, BuildTag, Error, Options};

const LOG_ENV_NAME: &'static str = "HYPER_LOG";

#[derive(Debug, StructOpt)]
/// Static site generator with mdbook support
#[structopt(name = "hypertext")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    /// Build tag name
    #[structopt(short, long)]
    tag: Option<String>,

    /// Follow symbolic links
    #[structopt(short, long)]
    follow_links: bool,

    /// Generate a release build
    #[structopt(short, long)]
    release: bool,

    /// Generate clean URLs
    #[structopt(short, long)]
    clean_url: bool,

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

fn fatal(e: impl std::error::Error) {
    error!("{}", e);
    std::process::exit(1);
}

fn error(s: String) {
    fatal(Error::new(s));
}

fn main() {
    let args = Cli::from_args();

    match &*args.log_level {
        "trace" => env::set_var(LOG_ENV_NAME, args.log_level),
        "debug" => env::set_var(LOG_ENV_NAME, args.log_level),
        "info" => env::set_var(LOG_ENV_NAME, args.log_level),
        "warn" => env::set_var(LOG_ENV_NAME, args.log_level),
        "error" => env::set_var(LOG_ENV_NAME, args.log_level),
        _ => {
            // Jump a few hoops to pretty print this message
            let level = &args.log_level;
            env::set_var(LOG_ENV_NAME, "error");
            pretty_env_logger::init_custom_env(LOG_ENV_NAME);
            error(format!("unknown log level: {}", level));
        }
    }

    pretty_env_logger::init_custom_env(LOG_ENV_NAME);

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

    // FIXME: remove minify option
    let minify = false;

    let mut tag = BuildTag::Debug;
    if args.release {
        //minify = true;
        tag = BuildTag::Release;
    }

    if let Some(t) = args.tag {
        if !t.is_empty() {
            tag = BuildTag::Custom(t);
        }
    }

    let target_dir = tag.get_path_name();

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

    let opts = Options {
        source: args.input,
        output: args.output,
        follow_links: args.follow_links,
        clean_url: args.clean_url,
        strict: !args.loose,
        release: args.release,
        target,
        minify,
        tag,
    };

    let now = SystemTime::now();
    if let Err(e) = build(opts) {
        fatal(e);
    }
    if let Ok(t) = now.elapsed() {
        info!("{:?}", t);
    }
}
