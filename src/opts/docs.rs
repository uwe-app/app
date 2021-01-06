use structopt::StructOpt;

use super::web_server::WebServerOpts;

#[derive(StructOpt, Debug)]
pub struct Docs {
    #[structopt(flatten)]
    pub server: WebServerOpts,

    /// Version range for the documentation plugin.
    #[structopt(env = "UWE_DOCS_VERSION_RANGE", hide_env_values = true)]
    pub version_range: Option<String>,
}
