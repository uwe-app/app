use std::env;
use std::path::PathBuf;

use log::info;

use config::server::{HostConfig, ServerConfig, TlsConfig};

use web_server::WebServerOpts;

use crate::{shim, Error, Result};

const LOG_ENV_NAME: &'static str = "UWE_LOG";
pub fn log_level(level: &str) -> Result<()> {
    match level {
        "trace" => env::set_var(LOG_ENV_NAME, level),
        "debug" => env::set_var(LOG_ENV_NAME, level),
        "info" => env::set_var(LOG_ENV_NAME, level),
        "warn" => env::set_var(LOG_ENV_NAME, level),
        "error" => env::set_var(LOG_ENV_NAME, level),
        _ => {
            // Jump a few hoops to pretty print this message
            env::set_var(LOG_ENV_NAME, "error");
            pretty_env_logger::init_custom_env(LOG_ENV_NAME);
            return Err(Error::UnknownLogLevel(level.to_string()));
        }
    }

    pretty_env_logger::init_custom_env(LOG_ENV_NAME);

    Ok(())
}

pub fn project_path(input: &PathBuf) -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;

    // NOTE: We want the help output to show "."
    // NOTE: to indicate that the current working
    // NOTE: directory is used but the period creates
    // NOTE: problems with the strip prefix logic for
    // NOTE: live reload so this converts it to the
    // NOTE: actual current working directory.
    let period = PathBuf::from(".");
    let result = if input == &period {
        cwd.clone()
    } else {
        input.clone()
    };

    if !result.exists() || !result.is_dir() {
        return Err(Error::NotDirectory(result));
    }

    let canonical = input.canonicalize()?;

    if canonical != cwd {
        let (mut local_version, mut version_file) =
            release::find_local_version(&canonical)?;
        let self_version = config::generator::semver();
        if let (Some(version), Some(version_file)) =
            (local_version.take(), version_file.take())
        {
            if &version != self_version {
                let bin_name = config::generator::bin_name();
                info!("Use version in {}", version_file.display());
                info!(
                    "Switch {} from {} to {}",
                    bin_name,
                    self_version.to_string(),
                    version.to_string()
                );
                shim::fork(bin_name, Some(version))?;
            }
        }
    }

    Ok(result)
}

pub fn tls_config(
    initial: Option<TlsConfig>,
    opts: &WebServerOpts,
    default_port_ssl: u16,
) -> Option<TlsConfig> {
    let mut tls = initial;

    let ssl_port = if let Some(ssl_port) = opts.ssl_port {
        ssl_port
    } else {
        default_port_ssl
    };

    if opts.ssl_cert.is_some() && opts.ssl_key.is_some() {
        let cert = opts.ssl_cert.as_ref().unwrap().to_path_buf();
        let key = opts.ssl_key.as_ref().unwrap().to_path_buf();
        let empty_cert = cert == PathBuf::from("");
        let empty_key = cert == PathBuf::from("");
        if !empty_cert && !empty_key {
            tls = Some(TlsConfig {
                cert,
                key,
                port: ssl_port,
            });
        }
    }

    tls
}

pub fn server_config(
    target: &PathBuf,
    opts: &WebServerOpts,
    default_port: u16,
    default_port_ssl: u16,
) -> ServerConfig {
    let host = opts.addr.clone();
    let mut port = &default_port;

    let tls = tls_config(Default::default(), opts, default_port_ssl);

    if let Some(ref p) = opts.port {
        port = p;
    }

    let host = HostConfig::new(
        target.clone(),
        host.to_owned(),
        None,
        None,
        false,
        false,
    );

    let mut server_config = ServerConfig::new_host(host, port.to_owned(), tls);
    server_config.listen = opts.addr.to_string();
    server_config.authorities = opts.authority.clone();
    server_config
}

mod alias;
mod build;
mod clean;
mod dev;
mod docs;
mod editor;
mod lang;
mod new;
mod publish;
mod server;
mod sync;
mod task;
mod test;
mod web_server;

pub mod uwe;

pub use self::alias::Alias;
pub use self::build::{Build, Compile};
pub use self::clean::Clean;
pub use self::dev::Dev;
pub use self::docs::Docs;
pub use self::editor::Editor;
pub use self::lang::Lang;
pub use self::new::New;
pub use self::publish::Publish;
pub use self::server::Server;
pub use self::sync::Sync;
pub use self::task::Task;
pub use self::test::Test;
