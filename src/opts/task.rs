use std::path::PathBuf;
use structopt::StructOpt;

use crate::opts::Alias;

/// Utility tasks.
#[derive(StructOpt, Debug)]
pub enum Task {
    /// Update runtime assets
    UpdateRuntimeAssets {},

    /// Update local documentation website
    UpdateDocs {},

    /// Update syntax highlight language definitions
    UpdateSyntax {},

    /// List project blueprints
    ListBlueprints {},

    /// Check project for local dependencies
    CheckDeps {
        /// Project path
        #[structopt(parse(from_os_str), default_value = ".")]
        project: PathBuf,
    },

    /// Manage site aliases (symbolic links)
    Alias {
        #[structopt(subcommand)]
        cmd: Alias,
    },
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
    /// Pull from repository.
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
}
