use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Sync {
    /*
    /// Initialize, add files and commit.
    Create {
        #[structopt(short, long)]
        message: String,

        /// Destination path.
        target: PathBuf,
    },

    /// Clone a repository.
    Clone {
        /// Repository URL.
        source: String,

        /// Destination path.
        target: Option<PathBuf>,
    },

    /// Copy a repository (clone and squash)
    Copy {
        /// Initial commit message.
        #[structopt(short, long)]
        message: String,

        /// Repository URL.
        source: String,

        /// Destination path.
        target: Option<PathBuf>,
    },
    */

    /*
    /// Pull a repository.
    Pull {
        #[structopt(short, long, default_value = "origin")]
        remote: String,

        #[structopt(short, long, default_value = "main")]
        branch: String,

        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },
    */

    /// Project path
    #[structopt(parse(from_os_str), default_value = ".")]
    pub project: PathBuf,
}
