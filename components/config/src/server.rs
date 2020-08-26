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

    // FIXME: use ServeConfig here
    pub host: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,

    pub open_browser: bool,
    pub watch: Option<PathBuf>,
    pub endpoint: String,
    pub redirects: Option<HashMap<String, Uri>>,

    /// Send headers that instruct browsers to disable caching.
    pub disable_cache: bool,
    /// When running a server over SSL redirect HTTP to HTTPS.
    pub redirect_insecure: bool,

    // TODO: support conditional logging
    pub log: bool,
    pub temporary_redirect: bool,
}

/// Determines the type of port to use.
pub enum PortType {
    Infer,
    Insecure,
    Secure,
}

impl ServeOptions {
    pub fn get_port(&self, port_type: PortType) -> u16 {
        match port_type {
            PortType::Infer => {
                if let Some(ref tls) = self.tls { tls.port } else { self.port }
            }
            PortType::Insecure => self.port,
            PortType::Secure => {
                if let Some(ref tls) = self.tls { tls.port } else { crate::PORT_SSL }
            }
        }
    }

    pub fn get_address(&self, port_type: PortType) -> String {
        let port = self.get_port(port_type);
        format!("{}:{}", self.host, port)
    }

    pub fn get_url(&self, scheme: &str, port_type: PortType) -> String {
        format!("{}//{}", scheme, self.get_address(port_type)) 
    }

    pub fn get_sock_addr(&self, port_type: PortType) -> Result<SocketAddr> {
        let address = self.get_address(port_type);
        Ok(address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::NoSocketAddress(address))?)
    }
}
