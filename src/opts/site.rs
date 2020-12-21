use std::path::PathBuf;
use structopt::StructOpt;

use super::alias::Alias;

/// Manage project source files
#[derive(StructOpt, Debug)]
pub enum Site {
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

    /// Pull a repository.
    Pull {
        #[structopt(short, long, default_value = "origin")]
        remote: String,

        #[structopt(short, long, default_value = "main")]
        branch: String,

        /// Repository path.
        target: Option<PathBuf>,
    },

    /// Manage site aliases
    Alias {
        #[structopt(flatten)]
        args: Alias,
    },
}
