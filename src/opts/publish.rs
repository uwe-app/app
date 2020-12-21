use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Publish {
    /// Publish environment
    #[structopt()]
    pub env: String,

    /// Allow hook command execution.
    #[structopt(short, long)]
    pub exec: bool,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,
}
