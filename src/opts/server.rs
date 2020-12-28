use std::path::PathBuf;

use structopt::StructOpt;

use super::build::Compile;
use super::web_server::WebServerOpts;

#[derive(StructOpt, Debug)]
pub struct Server {
    #[structopt(flatten)]
    pub server: WebServerOpts,

    /// Do not create a release build for projects
    #[structopt(short, long)]
    pub skip_build: bool,

    /// Launch a web browser
    #[structopt(short = "O", long)]
    pub open: bool,

    #[structopt(flatten)]
    pub build_opts: Compile,

    /// Project or directory to serve files from
    #[structopt(parse(from_os_str), default_value = ".")]
    pub target: PathBuf,
}
