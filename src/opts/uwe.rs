use structopt::StructOpt;

use super::{
    Build, Clean, Dev, Docs, Editor, Lang, New, Publish, Server, Sync, Task,
    Test,
};

#[derive(Debug, StructOpt)]
/// Universal web editor
#[structopt(
    name = "uwe",
    after_help = "EXAMPLES:
    Start a live reload server:
        uwe dev .
    Preview a release build:
        uwe server . --open
    Create a release build:
        uwe build .
    Browse offline help:
        uwe docs

Visit https://uwe.app for more guides and information.

To upgrade or uninstall use the version manager (uvm)."
)]
pub struct Uwe {
    /// Log level
    #[structopt(long, default_value = "info")]
    pub log_level: String,

    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Compile a site
    ///
    /// Creates a release build of the website into the `build/release` folder; use the `--profile`
    /// option to build to a different location with alternative build settings.
    ///
    /// If the project is a workspace all of the workspace members are compiled; filter the
    /// workspace members to build using the `--member` option.
    Build {
        #[structopt(flatten)]
        args: Build,
    },

    /// Launch a development server
    ///
    /// Compiles a debug build of the website into the `build/debug` folder and starts a web
    /// server with live reload enabled watching for changes to the source files in the `site`
    /// folder.
    Dev {
        #[structopt(flatten)]
        args: Dev,
    },

    /// Launch the editor user interface
    Editor {
        #[structopt(flatten)]
        args: Editor,
    },

    /// Remove the build directory
    Clean {
        #[structopt(flatten)]
        args: Clean,
    },

    /// Create a new project
    New {
        #[structopt(flatten)]
        args: New,
    },

    /// Utility tasks
    Task {
        #[structopt(subcommand)]
        cmd: Task,
    },

    /// Sync project source files
    Sync {
        #[structopt(flatten)]
        args: Sync,
    },

    /// Serve static files
    #[structopt(verbatim_doc_comment)]
    Server {
        #[structopt(flatten)]
        args: Server,
    },

    /// Browse the documentation
    Docs {
        #[structopt(flatten)]
        args: Docs,
    },

    /// Publish a website
    Publish {
        #[structopt(flatten)]
        args: Publish,
    },

    /// Manage translations
    Lang {
        #[structopt(subcommand)]
        cmd: Lang,
    },

    /// Run integration tests
    Test {
        #[structopt(flatten)]
        args: Test,
    },
}
