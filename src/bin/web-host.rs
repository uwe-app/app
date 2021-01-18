extern crate log;
extern crate pretty_env_logger;

use log::info;
use semver::Version;
use structopt::StructOpt;

use rusoto_core::Region;

use uwe::{
    self,
    opts::fatal,
    Error, Result,
};

use hosting::BucketHost;

fn parse_region(src: &str) -> std::result::Result<Region, Error> {
    src.parse::<Region>().map_err(Error::from)
}

/// Universal (web editor) plugin manager
#[derive(Debug, StructOpt)]
#[structopt(name = "upm")]
struct Cli {
    /// Log level
    #[structopt(long, default_value = "info")]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Bucket {
    /// Ensure a bucket is available
    Up {
        /// Credentials profile name
        #[structopt(short, long)]
        credentials: String,

        /// Bucket region
        #[structopt(short, long, parse(try_from_str = parse_region))]
        region: Region,

        /// Bucket name
        bucket: String,
    },
}

#[derive(StructOpt, Debug)]
enum Command {

    /// Bucket commands
    #[structopt(alias = "b")]
    Bucket {
        #[structopt(subcommand)]
        cmd: Bucket,
    },
}

async fn run(cmd: Command) -> Result<()> {
    match cmd {
        Command::Bucket { cmd } => {
            match cmd {
                Bucket::Up { credentials, region, bucket } => {
                    let client = hosting::new_client(&credentials, &region)?;
                    let bucket_host = BucketHost::new(region, bucket);
                    bucket_host.up(&client).await?;
                }
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
    let bin_name = env!("CARGO_BIN_NAME").to_string();
    let user_agent = format!("{}/{}", &name, &version);
    let semver: Version = version.parse().unwrap();

    info!("{}", &version);

    let app_data = config::generator::AppData {
        name,
        bin_name,
        version,
        user_agent,
        semver,
    };
    config::generator::get(Some(app_data));

    Ok(run(args.cmd).await.map_err(Error::from).or_else(fatal)?)
}
