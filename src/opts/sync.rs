use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Sync {
    /// Remote for the sync operation
    #[structopt(short, long)]
    pub remote: Option<String>,

    /// Branch for the sync operation
    #[structopt(short, long)]
    pub branch: Option<String>,

    /// Commit message
    #[structopt(short, long)]
    pub message: Option<String>,

    /// Add untracked files
    #[structopt(short, long)]
    pub add: bool,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,
}
