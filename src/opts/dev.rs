use std::path::PathBuf;
use structopt::StructOpt;

use config::ProfileName;

use super::{build::Compile, web_server::WebServerOpts};

#[derive(StructOpt, Debug)]
pub struct Dev {
    #[structopt(flatten)]
    pub compile: Compile,

    /// Build profile name
    #[structopt(long, default_value = "debug")]
    pub profile: ProfileName,

    /// Launch page URL
    #[structopt(long)]
    pub launch: Option<String>,

    /// Do not launch a browser
    #[structopt(long)]
    pub headless: bool,

    #[structopt(flatten)]
    pub server: WebServerOpts,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,

    /// Compile only these paths
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}
