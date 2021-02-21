use structopt::StructOpt;

use config::ProfileName;

use super::{build::Compile, web_server::WebServerOpts};

#[derive(StructOpt, Debug)]
pub struct Editor {
    #[structopt(flatten)]
    pub compile: Compile,

    /// Build profile name
    #[structopt(long, default_value = "debug")]
    pub profile: ProfileName,

    #[structopt(flatten)]
    pub server: WebServerOpts,
}
