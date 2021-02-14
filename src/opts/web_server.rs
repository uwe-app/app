use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct WebServerOpts {
    /// Bind address for the web server
    #[structopt(short, long, default_value = "0.0.0.0")]
    pub addr: String,

    /// The port number
    #[structopt(short, long)]
    pub port: Option<u16>,

    /// Allow these virtual host authorities.
    #[structopt(long)]
    pub authority: Option<Vec<String>>,

    /// The port number for SSL
    #[structopt(long)]
    pub ssl_port: Option<u16>,

    /// Path to an SSL certificate file
    #[structopt(long, env = "UWE_SSL_CERT", hide_env_values = true)]
    pub ssl_cert: Option<PathBuf>,

    /// Path to an SSL key file
    #[structopt(long, env = "UWE_SSL_KEY", hide_env_values = true)]
    pub ssl_key: Option<PathBuf>,
}
