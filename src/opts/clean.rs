use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Clean {
    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,
}
