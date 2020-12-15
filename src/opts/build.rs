use std::path::PathBuf;

use structopt::StructOpt;

use super::web_server::WebServerOpts;

#[derive(StructOpt, Debug)]
pub struct Build {
    /// Build profile name
    #[structopt(long)]
    pub profile: Option<String>,

    /// Offline mode, do not attempt plugin installation
    #[structopt(short, long)]
    pub offline: bool,

    /// Enable live reload
    #[structopt(short, long)]
    pub live: bool,

    /// Launch path for a page URL (live reload)
    #[structopt(long)]
    pub launch: Option<String>,

    /// Generate a release build
    #[structopt(short, long)]
    pub release: bool,

    #[structopt(flatten)]
    pub server: WebServerOpts,

    /// Read config from directory
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,

    /// Compile only these paths
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}
