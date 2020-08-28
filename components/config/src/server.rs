use std::net::{SocketAddr, ToSocketAddrs};
use std::collections::HashMap;
use std::path::PathBuf;

use http::Uri;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug)]
pub struct ConnectionInfo {
    pub addr: SocketAddr,
    pub host: String,
    pub tls: bool,
}

impl ConnectionInfo {
    pub fn to_url(&self) -> String {
        let scheme = if self.tls { crate::SCHEME_HTTPS } else { crate::SCHEME_HTTP };
        crate::to_url_string(scheme, &self.host, self.addr.port())
    }

    pub fn to_websocket_url(&self, endpoint: &str) -> String {
        let scheme = if self.tls { crate::SCHEME_WSS } else { crate::SCHEME_WS };
        format!("{}//{}:{}/{}", scheme, &self.host, self.addr.port(), endpoint)
    }
}

/// Determines the type of port to use.
pub enum PortType {
    Infer,
    Insecure,
    Secure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert: PathBuf,
    pub key: PathBuf,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub open: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,

    #[serde(skip)]
    pub target: PathBuf,

    //#[serde(skip)]
    //pub open_browser: bool,
    #[serde(skip)]
    pub watch: Option<PathBuf>,
    #[serde(skip)]
    pub endpoint: String,
    #[serde(skip)]
    pub redirects: Option<HashMap<String, Uri>>,

    /// Send headers that instruct browsers to disable caching.
    #[serde(skip)]
    pub disable_cache: bool,
    /// When running a server over SSL redirect HTTP to HTTPS.
    #[serde(skip)]
    pub redirect_insecure: bool,

    #[serde(skip)]
    pub temporary_redirect: bool,

    #[serde(skip)]
    pub log: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            target: PathBuf::from(""),

            host: String::from(crate::config::HOST),
            port: crate::config::PORT,
            tls: None,

            watch: None,
            endpoint: "".to_string(),
            redirects: None,
            disable_cache: false,
            redirect_insecure: false,
            temporary_redirect: false,
            log: false,
        }
    }
}

impl ServerConfig {
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

