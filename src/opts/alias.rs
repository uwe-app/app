use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Alias {
    /// Add a site
    Add {
        /// Project folder
        #[structopt(parse(from_os_str))]
        project: PathBuf,

        /// Project name
        name: Option<String>,
    },

    /// Remove a site
    #[structopt(alias = "rm")]
    Remove {
        /// The project name
        name: String,
    },

    /// List sites
    #[structopt(alias = "ls")]
    List {},
}
