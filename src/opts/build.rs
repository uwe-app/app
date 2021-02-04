use std::path::PathBuf;

use structopt::StructOpt;

use super::web_server::WebServerOpts;

use config::ProfileName;

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

#[derive(StructOpt, Debug)]
pub struct Build {
    #[structopt(flatten)]
    pub compile: Compile,

    /// Build profile name
    #[structopt(long, default_value = "release")]
    pub profile: ProfileName,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,

    /// Compile only these paths
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}

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

    #[structopt(flatten)]
    pub server: WebServerOpts,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,

    /// Compile only these paths
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}
