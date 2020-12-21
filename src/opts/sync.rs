use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Sync {
    #[structopt(short, long)]
    pub remote: Option<String>,

    #[structopt(short, long)]
    pub branch: Option<String>,

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,
}
