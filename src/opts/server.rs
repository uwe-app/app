use std::path::PathBuf;

use structopt::StructOpt;

use super::build::Compile;
use super::web_server::WebServerOpts;

#[derive(StructOpt, Debug)]
pub struct Server {
    #[structopt(flatten)]
    pub server: WebServerOpts,

    /// Launch a web browser
    #[structopt(short = "O", long)]
    pub open: bool,

    #[structopt(flatten)]
    pub build_opts: Compile,

    /// Directory to serve
    #[structopt(short, long, parse(from_os_str))]
    pub directory: Option<PathBuf>,

    /// Config file
    #[structopt(short, long, parse(from_os_str))]
    pub config: Option<Vec<PathBuf>>,

    /// Project path
    #[structopt(parse(from_os_str))]
    pub project: Option<PathBuf>,
}
