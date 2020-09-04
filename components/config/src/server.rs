use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::redirect::Redirects;

use crate::{Error, Result};

pub fn to_websocket_url(tls: bool, host: &str, endpoint: &str, port: u16) -> String {
    let scheme = if tls { crate::SCHEME_WSS } else { crate::SCHEME_WS };
    format!("{}//{}:{}/{}", scheme, host, port, endpoint)
}

pub fn get_port(port: u16, tls: &Option<TlsConfig>, port_type: PortType) -> u16 {
    match port_type {
        PortType::Infer => {
            if let Some(ref tls) = tls { tls.port } else { port }
        }
        PortType::Insecure => port,
        PortType::Secure => {
            if let Some(ref tls) = tls { tls.port } else { crate::PORT_SSL }
        }
    }
}

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
        to_websocket_url(self.tls, &self.host, endpoint, self.addr.port())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub prefix: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {prefix: "web:log".to_string()}
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub listen: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
    pub default_host: HostConfig,
    pub hosts: Vec<HostConfig>,

    /// When running a server over SSL redirect HTTP to HTTPS.
    #[serde(skip)]
    pub redirect_insecure: bool,
    /// Whether redirects should use a temporary status code.
    #[serde(skip)]
    pub temporary_redirect: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: String::from(crate::config::HOST),
            port: crate::config::PORT,
            tls: None,
            redirect_insecure: true,
            temporary_redirect: false,
            default_host: Default::default(),
            hosts: vec![],
        }
    }
}

impl ServerConfig {

    /// New configuration for a host and port.
    pub fn new(listen: String, port: u16, tls: Option<TlsConfig>) -> Self {
        let mut tmp: Self = Default::default();
        tmp.listen = listen;
        tmp.port = port;
        tmp.tls = tls;
        tmp
    }

    /// New configuration using a default host.
    pub fn new_host(host: HostConfig, port: u16, tls: Option<TlsConfig>) -> Self {
        Self {
            listen: String::from(crate::config::HOST),
            port: port,
            tls,
            redirect_insecure: true,
            temporary_redirect: true,
            default_host: host,
            hosts: vec![],
        }
    }

    pub fn get_port(&self, port_type: PortType) -> u16 {
        get_port(self.port, &self.tls, port_type)
    }

    pub fn get_address(&self, port_type: PortType, host: Option<String>) -> String {
        let port = self.get_port(port_type);
        let host = if let Some(host) = host {
            host.clone()
        } else {
            self.listen.clone()
        };
        format!("{}:{}", host, port)
    }

    pub fn get_url(&self, scheme: &str, port_type: PortType, host: Option<String>) -> String {
        format!("{}//{}", scheme, self.get_address(port_type, host)) 
    }

    pub fn get_sock_addr(&self, port_type: PortType, host: Option<String>) -> Result<SocketAddr> {
        let address = self.get_address(port_type, host);
        Ok(address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::NoSocketAddress(address))?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostConfig {
    pub directory: PathBuf,
    pub name: String,
    #[serde(skip)]
    pub redirects: Option<Redirects>,
    #[serde(skip)]
    pub endpoint: Option<String>,
    /// Send headers that instruct browsers to disable caching.
    #[serde(skip)]
    pub disable_cache: bool,
    #[serde(skip)]
    pub log: Option<LogConfig>,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            name: crate::config::HOST.to_string(),
            directory: PathBuf::from(""),
            redirects: None,
            endpoint: None,
            disable_cache: false,
            log: None,
        }
    }
}

impl HostConfig {
    pub fn new(
        directory: PathBuf,
        name: String,
        redirects: Option<Redirects>,
        endpoint: Option<String>) -> Self {

        Self {
            directory,
            name,
            redirects,
            endpoint,
            disable_cache: true,
            log: None,
        } 
    }
}
