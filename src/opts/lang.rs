use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Lang {
    /// List languages for a project
    #[structopt(alias = "ls")]
    List {
        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },

    /// Create new translations
    New {
        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,

        /// Unicode language identifiers
        languages: Vec<String>,
    },
}
