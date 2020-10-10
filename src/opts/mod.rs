use std::path::PathBuf;
use std::panic;

use log::error;

use config::{
    server::{HostConfig, ServerConfig, TlsConfig},
};

use web_server::WebServerOpts;

use crate::{Error, Result};

pub fn server_config(
    target: &PathBuf,
    opts: &WebServerOpts,
    default_port: u16,
    default_port_ssl: u16,
) -> ServerConfig {
    let serve: ServerConfig = Default::default();
    let mut host = &serve.listen;
    let mut port = &default_port;
    let mut tls = serve.tls.clone();

    let ssl_port = if let Some(ssl_port) = opts.ssl_port {
        ssl_port
    } else {
        default_port_ssl
    };

    if let Some(ref h) = opts.host {
        host = h;
    }
    if let Some(ref p) = opts.port {
        port = p;
    }

    if opts.ssl_cert.is_some() && opts.ssl_key.is_some() {
        tls = Some(TlsConfig {
            cert: opts.ssl_cert.as_ref().unwrap().to_path_buf(),
            key: opts.ssl_key.as_ref().unwrap().to_path_buf(),
            port: ssl_port,
        });
    }

    let host = HostConfig::new(target.clone(), host.to_owned(), None, None);

    ServerConfig::new_host(host, port.to_owned(), tls)
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

pub fn fatal(e: Error) -> Result<()> {
    print_error(e);
    std::process::exit(1);
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

pub mod build;
pub mod docs;
pub mod init;
pub mod publish;
pub mod run;
pub mod site;
pub mod web_server;

pub use self::build::Build;
pub use self::docs::Docs;
pub use self::init::Init;
pub use self::publish::Publish;
pub use self::run::Run;
pub use self::site::Site;
