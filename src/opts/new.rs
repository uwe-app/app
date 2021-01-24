use std::path::PathBuf;

use structopt::StructOpt;
use url::Url;

#[derive(StructOpt, Debug)]
pub struct New {
    /// Initial commit message.
    #[structopt(short, long)]
    pub message: Option<String>,

    /// Language for the new project
    #[structopt(short, long)]
    pub language: Option<String>,

    /// Host name for the new project
    #[structopt(short, long)]
    pub host: Option<String>,

    /// Create translation locales (comma delimited)
    #[structopt(short = "L", long)]
    pub locales: Option<String>,

    /// Remote name for the new project
    #[structopt(long, default_value = "origin")]
    pub remote_name: String,

    /// Remote repository URL for the new project
    #[structopt(short, long)]
    pub remote_url: Option<String>,

    /// Create project from a git blueprint
    #[structopt(short, long)]
    pub git: Option<Url>,

    /// Create project from a folder blueprint
    #[structopt(short, long, parse(from_os_str))]
    pub path: Option<PathBuf>,

    /// Output directory for the new project
    #[structopt(parse(from_os_str))]
    pub target: PathBuf,

    /// Plugin name for the project blueprint
    ///
    /// If no plugin name is specified the default
    /// plugin will be used. If a bare name is given
    /// then it is assumed to be in the std::blueprint
    /// namespace.
    #[structopt()]
    pub plugin: Option<String>,
}
