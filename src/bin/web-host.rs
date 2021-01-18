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

use web_host::BucketHost;

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
        /// Suffix for folder requests
        #[structopt(short, long, default_value = "index.html")]
        index_suffix: String,

        /// Key for a bucket error handler
        #[structopt(short, long, default_value = "404.html")]
        error_key: String,

        /// Redirect all requests to the given host name
        #[structopt(long)]
        redirect_host_name: Option<String>,

        /// Use the given protocol when redirecting requests 
        #[structopt(long)]
        redirect_protocol: Option<String>,

        /// Credentials profile name
        #[structopt(short, long)]
        credentials: String,

        /// Region for the bucket
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
                Bucket::Up {
                    credentials,
                    region,
                    bucket,
                    index_suffix,
                    error_key,
                    redirect_host_name,
                    redirect_protocol,
                } => {
                    let client = web_host::new_client(&credentials, &region)?;
                    let bucket_host = BucketHost::new(
                        region,
                        bucket,
                        index_suffix,
                        error_key,
                        redirect_host_name,
                        redirect_protocol,
                    );
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
