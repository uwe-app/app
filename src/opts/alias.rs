use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Alias {
    /// Add an alias
    Add {
        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,

        /// Project name
        name: Option<String>,
    },

    /// Remove an alias
    #[structopt(alias = "rm")]
    Remove {
        /// The project name
        name: String,
    },

    /// List aliases
    #[structopt(alias = "ls")]
    List {},
}
