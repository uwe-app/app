use std::path::PathBuf;
use structopt::StructOpt;

/// Utility tasks.
#[derive(StructOpt, Debug)]
pub enum Task {
    /// List blueprints.
    ListBlueprints {},

    /// Check project for local dependencies.
    CheckDeps {
        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },
}
