use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct WebServerOpts {
    /// The name of the host
    #[structopt(short, long)]
    pub host: Option<String>,

    /// The port number
    #[structopt(short, long)]
    pub port: Option<u16>,

    /// The port number for SSL
    #[structopt(long)]
    pub ssl_port: Option<u16>,

    /// Path to an SSL certificate file
    #[structopt(long, env, hide_env_values = true)]
    pub ssl_cert: Option<PathBuf>,

    /// Path to an SSL key file
    #[structopt(long, env, hide_env_values = true)]
    pub ssl_key: Option<PathBuf>,
}
