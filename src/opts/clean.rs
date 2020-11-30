use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Clean {
    /// Read config from directory
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,
}
