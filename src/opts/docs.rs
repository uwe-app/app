use structopt::StructOpt;

use super::web_server::WebServerOpts;

#[derive(StructOpt, Debug)]
pub struct Docs {
    #[structopt(flatten)]
    pub server: WebServerOpts,
}
