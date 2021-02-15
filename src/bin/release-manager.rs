extern crate log;
extern crate pretty_env_logger;

use structopt::StructOpt;

use uwe::{fatal, Error, Result};

/// Package and publish a release.
#[derive(Debug, StructOpt)]
#[structopt(name = "uwe-release")]
struct Cli {
    /// The bucket name.
    #[structopt(short, long, default_value = "releases.uwe.app")]
    bucket: String,

    /// The bucket region.
    #[structopt(short, long, default_value = "ap-southeast-1")]
    region: String,

    /// The credentials profile name.
    #[structopt(short, long, default_value = "uwe")]
    profile: String,

    /// Skip the build step.
    #[structopt(long)]
    skip_build: bool,

    /// Skip the test step.
    #[structopt(long)]
    skip_test: bool,

    /// Skip compiling the cookbook.
    #[structopt(long)]
    skip_cookbook: bool,

    /// Skip compiling the build tools.
    #[structopt(long)]
    skip_build_tools: bool,

    /// Skip the upload step.
    #[structopt(long)]
    skip_upload: bool,

    /// Force overwrite an existing version.
    #[structopt(short, long)]
    force: bool,
}

/// Create a release for the current version.
#[tokio::main]
async fn main() -> Result<()> {
    let root_args = Cli::from_args();

    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    // Must configure the version here otherwise option_env!() will
    // use the version from the workspace package which we don't really
    // care about, the top-level version is the one that interests us.
    let manifest = option_env!("CARGO_MANIFEST_DIR").unwrap().to_string();
    let name = option_env!("CARGO_PKG_NAME").unwrap().to_string();
    let version = option_env!("CARGO_PKG_VERSION").unwrap().to_string();

    release::publish(
        manifest,
        name,
        version,
        root_args.bucket,
        root_args.region,
        root_args.profile,
        root_args.skip_build,
        root_args.skip_test,
        root_args.skip_cookbook,
        root_args.skip_build_tools,
        root_args.skip_upload,
        root_args.force,
    )
    .await
    .map_err(Error::from)
    .or_else(fatal)?;

    Ok(())
}
