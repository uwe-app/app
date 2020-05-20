extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::env;
use std::path::PathBuf;
use structopt::StructOpt;
use std::fs;
use log::{info};

use hypertext::{
    build,
    Options
};

const LOG_ENV_NAME: &'static str = "HYPER_LOG";

#[derive(Debug, StructOpt)]
/// Static site generator with mdbook support
#[structopt(name = "hypertext")]
struct Cli {

    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    /// Follow symbolic links
    #[structopt(short, long)]
    follow_links: bool,

    /// Compress HTML output
    #[structopt(short, long)]
    minify: bool,

    /// Generate clean URLs
    #[structopt(short, long)]
    clean_url: bool,

    /// Read files from directory
    #[structopt(parse(from_os_str), default_value="site")]
    input: PathBuf,

    /// Write files to directory
    #[structopt(parse(from_os_str), default_value="build")]
    output: PathBuf,
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
            error!("unknown log level: {}", level);
            std::process::exit(1);
        },
    }

    pretty_env_logger::init_custom_env(LOG_ENV_NAME);

    if !args.input.is_dir() {
        error!("not a directory: {}", args.input.display());
        std::process::exit(1);
    }

    if !args.output.exists() {
        info!("mkdir {}", args.output.display());
        if let Err(e) = fs::create_dir(&args.output) {
            error!("{}", e);
            std::process::exit(1);
        }
    }

    if !args.output.is_dir() {
        error!("not a directory: {}", args.output.display());
        std::process::exit(1);
    }

    let opts = Options{
        source: args.input, 
        target: args.output,
        follow_links: args.follow_links,
        clean_url: args.clean_url,
        minify: args.minify,
    };

    build(opts);
}
