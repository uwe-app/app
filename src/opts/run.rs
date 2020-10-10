use std::path::PathBuf;

use structopt::StructOpt;

use super::web_server::WebServerOpts;

#[derive(StructOpt, Debug)]
pub struct Run {
    #[structopt(flatten)]
    pub server: WebServerOpts,

    /// Directory to serve files from
    #[structopt(parse(from_os_str))]
    pub target: PathBuf,
}
