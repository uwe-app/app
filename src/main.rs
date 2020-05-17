extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::env;
use std::path::PathBuf;
use structopt::StructOpt;
use regex::Regex;
use std::fs;
use log::{info};

use hypertext::{Finder, InputOptions, OutputOptions};
use hypertext::matcher::{FileMatcher};

const LOG_ENV_NAME: &'static str = "HYPER_LOG";

#[derive(Debug, StructOpt)]
/// Static site generator with mdbook support
#[structopt(name = "hypertext")]
struct Cli {

    /// Theme directory used for books
    #[structopt(long)]
    theme: Option<String>,

    /// Log level
    #[structopt(long, short,default_value = "info")]
    log_level: String,

    /// Follow symbolic links
    #[structopt(long)]
    follow_links: bool,

    /// Ignore patterns
    #[structopt(short, long)]
    ignore: Option<Vec<Regex>>,

    /// Read files from directory
    #[structopt(parse(from_os_str), default_value="site")]
    input: PathBuf,

    /// Write files to directory
    #[structopt(parse(from_os_str), default_value="build")]
    output: PathBuf,
}

fn main() {
    let args = Cli::from_args();
    //println!("hypertext(1) {:?}", args.ignore);

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

    let mut def_build = PathBuf::new();
    def_build.push("build");

    if args.output == def_build && !args.output.exists() {
        info!("mkdir {}", def_build.display());
        if let Err(e) = fs::create_dir(def_build) {
            error!("{}", e);
            std::process::exit(1);
        }
    }

    if !args.output.is_dir() {
        error!("not a directory: {}", args.output.display());
        std::process::exit(1);
    }

    let matcher = FileMatcher::new(args.ignore);
    let input_opts = InputOptions{
        source: args.input.clone(), 
        follow_links: args.follow_links,
        matcher: matcher,
    };

    let output_opts = OutputOptions{
        target: args.output.clone(),
        theme: args.theme.unwrap_or("".to_string()),
    };

    let finder = Finder::new(input_opts, output_opts);
    finder.run();
}
