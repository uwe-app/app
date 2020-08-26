use std::net::{SocketAddr, ToSocketAddrs};
use std::collections::HashMap;
use std::path::PathBuf;

use http::Uri;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            host: String::from(crate::config::HOST),
            port: crate::config::PORT,
            tls: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert: PathBuf,
    pub key: PathBuf,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub target: PathBuf,
    pub host: String,
    pub port: u16,
    pub open_browser: bool,
    pub tls: Option<TlsConfig>,
    pub watch: Option<PathBuf>,
    pub endpoint: String,
    pub redirects: Option<HashMap<String, Uri>>,

    pub disable_cache: bool,

    // TODO: support conditional logging
    pub log: bool,
    pub temporary_redirect: bool,
}

impl ServeOptions {
    pub fn get_port(&self) -> u16 {
        if let Some(ref tls) = self.tls { tls.port } else { self.port }
    }

    pub fn get_address(&self) -> String {
        let port = self.get_port();
        format!("{}:{}", self.host, port)
    }

    pub fn get_sock_addr(&self) -> Result<SocketAddr> {
        let address = self.get_address();
        Ok(address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::NoSocketAddress(address))?)
    }
}
