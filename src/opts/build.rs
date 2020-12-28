use std::path::PathBuf;

use structopt::StructOpt;

use super::web_server::WebServerOpts;

/// Common compiler options for the `build` and `server` commands.
#[derive(StructOpt, Debug)]
pub struct Compile {
    /// Allow hook command execution.
    #[structopt(short, long)]
    pub exec: bool,

    /// Include drafts
    #[structopt(short, long)]
    pub include_drafts: bool,

    /// Filter on workspace members
    #[structopt(short, long)]
    pub member: Vec<String>,
}

/// Options for the `build` command.
#[derive(StructOpt, Debug)]
pub struct Build {
    #[structopt(flatten)]
    pub compile: Compile,

    /// Build profile name
    #[structopt(long)]
    pub profile: Option<String>,

    /// Offline mode, do not attempt plugin installation
    #[structopt(short, long)]
    pub offline: bool,

    /// Launch path for a page URL (live reload)
    #[structopt(long)]
    pub launch: Option<String>,

    /// Enable live reload
    #[structopt(short, long)]
    pub live: bool,

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
