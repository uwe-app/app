use std::path::PathBuf;

use structopt::StructOpt;

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

    /// Remote name
    #[structopt(long, default_value = "origin")]
    pub remote_name: String,

    /// Remote repository URL
    #[structopt(short, long)]
    pub remote_url: Option<String>,

    /// Output directory for the new project
    #[structopt(parse(from_os_str))]
    pub target: PathBuf,

    /// Repository URL, folder or blueprint name.
    #[structopt()]
    pub source: Option<String>,
}
