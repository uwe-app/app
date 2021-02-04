use std::path::PathBuf;
use structopt::StructOpt;

use super::build::Compile;
use super::web_server::WebServerOpts;

/// Run integration tests.
#[derive(StructOpt, Debug)]
pub struct Test {
    #[structopt(flatten)]
    pub server: WebServerOpts,

    #[structopt(flatten)]
    pub build_opts: Compile,

    /// Build profile name
    #[structopt(long)]
    pub profile: Option<String>,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,
}
