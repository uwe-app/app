extern crate log;
extern crate pretty_env_logger;

use std::path::PathBuf;

use log::error;
use structopt::StructOpt;

use bundler::{BundleError, BundleOptions};

fn fatal(e: impl std::error::Error) {
    error!("{}", e);
    std::process::exit(1);
}

#[derive(Debug, StructOpt)]
#[structopt(name = "ht-bundle", version = "1.0.0")]
struct Cli {
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
    #[structopt(parse(from_os_str), default_value = "build/bundle")]
    output: PathBuf,
}

fn main() {
    let args = Cli::from_args();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    if !args.input.exists() || !args.input.is_dir() {
        fatal(BundleError::NotDirectory(args.input));
        return;
    }

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

    if let Err(e) = bundler::bundle(opts) {
        fatal(e);
    }
}
