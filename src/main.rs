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

    /// Specific theme directory used for books
    ///
    /// Overrides the theme directory convention.
    #[structopt(long)]
    theme: Option<String>,

    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    /// Layout file name
    #[structopt(long, default_value = "layout.hbs")]
    layout: String,

    /// Follow symbolic links
    #[structopt(long)]
    follow_links: bool,

    /// Exclude patterns
    ///
    /// Any paths matching these regular expression patterns are excluded 
    /// from processing.
    ///
    /// Match is performed on the entire file path.
    ///
    /// The file path may be relative or absolute depending upon the input.
    ///
    #[structopt(short, long)]
    exclude: Option<Vec<Regex>>,

    /// Read files from directory
    #[structopt(parse(from_os_str), default_value="site")]
    input: PathBuf,

    /// Write files to directory
    #[structopt(parse(from_os_str), default_value="build")]
    output: PathBuf,
}

fn main() {
    let args = Cli::from_args();
    //println!("hypertext(1) {:?}", args.exclude);

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

    let input_opts = InputOptions{
        matcher: FileMatcher::new(args.exclude.clone(), args.layout.clone()),
        layout: args.layout.clone(),
        source: args.input.clone(), 
        follow_links: args.follow_links,
        templates: "template".to_string(),
    };

    let output_opts = OutputOptions{
        matcher: FileMatcher::new(args.exclude.clone(), args.layout.clone()),
        target: args.output.clone(),
        theme: args.theme.unwrap_or("".to_string()),
        clean: true,
    };

    let finder = Finder::new(input_opts, output_opts);
    finder.run();
}
