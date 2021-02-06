use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};

use super::build::Compile;
use super::web_server::WebServerOpts;

use config::ProfileName;

/// Run integration tests.
#[derive(StructOpt, Debug)]
#[structopt(
    //setting = AppSettings::AllowMissingPositional,
    setting = AppSettings::TrailingVarArg,
)]
pub struct Test {
    #[structopt(flatten)]
    pub server: WebServerOpts,

    #[structopt(flatten)]
    pub build_opts: Compile,

    /// Build profile name
    #[structopt(long, default_value = "test")]
    pub profile: ProfileName,

    /// Project path
    #[structopt(
        parse(from_os_str),
        default_value = ".",
        multiple = true,
        number_of_values = 1
    )]
    pub project: PathBuf,
}
