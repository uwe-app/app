use std::panic;
use std::path::PathBuf;

use log::error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Panic(String),

    #[error("Unknown log level {0}")]
    UnknownLogLevel(String),

    #[error("Not a directory {0}")]
    NotDirectory(PathBuf),

    #[error("Not a file {0}")]
    NotFile(PathBuf),

    #[error("Path {0} error ({1})")]
    PathIo(PathBuf, String),

    #[error("Path {0} is absolute but a relative path is required")]
    NotRelative(PathBuf),

    #[error("Refusing to overwrite {0}, please move it away to initialize integration tests")]
    NoOverwriteTestSpec(PathBuf),

    #[error("No publish configuration")]
    NoPublishConfiguration,

    #[error("Unknown publish environment {0}")]
    UnknownPublishEnvironment(String),

    #[error("Plugin publishing is not available yet")]
    NoPluginPublishPermission,

    #[error("Could not determine a target name")]
    NoTargetName,

    #[error("Repository {0} is not a valid URL")]
    InvalidRepositoryUrl(String),

    #[error("Not a repository {0}")]
    NotRepository(PathBuf),

    #[error("Server could not find {0} or {1} in {2}")]
    NoServerFile(String, String, PathBuf),

    #[error("Local dependency {0} is not allowed")]
    LocalDependencyNotAllowed(PathBuf),

    #[error("Plugin {0}@{1} is already installed, use --force to overwrite")]
    PluginAlreadyInstalled(String, String),

    #[error("To add a plugin requires a name, path, archive or URL.")]
    PluginAddNoTarget,

    #[error("Plugin targets cannot be mixed; use a plugin name or an option (--path, --archive, --git)")]
    PluginAddMultipleTargets,

    #[error("Failed to spawn the command '{0}', check the program is installed and has executable permissions")]
    CommandSpawn(String),

    #[error("The command '{0}' failed, see it's error output")]
    CommandExec(String),

    #[error("Test runner command '{0} {1}' failed, see it's error output for more details")]
    IntegrationTestFail(String, String),

    #[error(
        "The server comamnd expects a project path or --directory or --config"
    )]
    NoServerTargets,

    #[error("The server comamnd expects either a project path, a directory or a config but multiple options given")]
    TooManyServerTargets,

    #[error("No editor directory, please set UWE_EDITOR")]
    NoEditorDirectory,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    ReqParse(#[from] semver::ReqParseError),

    #[error(transparent)]
    LanguageIdentifier(#[from] unic_langid::LanguageIdentifierError),

    #[error(transparent)]
    ParseRegion(#[from] rusoto_core::region::ParseRegionError),

    #[error(transparent)]
    Recv(#[from] tokio::sync::oneshot::error::RecvError),

    #[error(transparent)]
    Psup(#[from] psup_impl::Error),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Compiler(#[from] compiler::Error),

    #[error(transparent)]
    Locale(#[from] locale::Error),

    #[error(transparent)]
    Workspace(#[from] workspace::Error),

    #[error(transparent)]
    Scm(#[from] scm::Error),

    #[error(transparent)]
    Preference(#[from] preference::Error),

    #[error(transparent)]
    Publish(#[from] publisher::Error),

    #[error(transparent)]
    Project(#[from] project::Error),

    #[error(transparent)]
    Server(#[from] server::Error),

    #[error(transparent)]
    Plugin(#[from] plugin::Error),

    #[error(transparent)]
    Utils(#[from] utils::Error),

    #[error(transparent)]
    Release(#[from] release::Error),

    #[error(transparent)]
    Shim(#[from] crate::shim::Error),
}

pub fn panic_hook() {
    // Fluent templates panics if an error is caught parsing the
    // templates (for example attempting to override from a shared resource)
    // so we catch it here and push it out via the log
    panic::set_hook(Box::new(|info| {
        let message = format!("{}", info);
        // NOTE: We must NOT call `fatal` here which explictly exits the program;
        // NOTE: if we did our defer! {} hooks would not get called which means
        // NOTE: lock files would not be removed from disc correctly.
        print_error(Error::Panic(message));
    }));
}

fn compiler_error(e: &compiler::Error) {
    match e {
        compiler::Error::Multi { ref errs } => {
            error!("Compile error ({})", errs.len());
            for e in errs {
                error!("{}", e);
            }
            std::process::exit(1);
        }
        _ => {}
    }

    error!("{}", e);
}

pub fn print_error(e: Error) {
    match e {
        Error::Compiler(ref e) => {
            return compiler_error(e);
        }
        Error::Workspace(ref e) => match e {
            workspace::Error::Compiler(ref e) => {
                return compiler_error(e);
            }
            _ => {}
        },
        _ => {}
    }
    error!("{}", e);
}

pub fn fatal(e: Error) -> Result<(), Error> {
    print_error(e);
    std::process::exit(1);
}

pub fn server_error_cb(e: server::Error) {
    let _ = fatal(Error::from(e));
}
