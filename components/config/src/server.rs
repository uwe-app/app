use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::redirect::Redirects;

use crate::{Error, Result};

pub fn to_websocket_url(
    tls: bool,
    host: &str,
    endpoint: &str,
    port: u16,
) -> String {
    let scheme = if tls {
        crate::SCHEME_WSS
    } else {
        crate::SCHEME_WS
    };
    format!("{}//{}:{}/{}", scheme, host, port, endpoint)
}

pub fn get_port(
    port: u16,
    tls: &Option<TlsConfig>,
    port_type: PortType,
) -> u16 {
    match port_type {
        PortType::Infer => {
            if let Some(ref tls) = tls {
                tls.port
            } else {
                port
            }
        }
        PortType::Insecure => port,
        PortType::Secure => {
            if let Some(ref tls) = tls {
                tls.port
            } else {
                crate::PORT_SSL
            }
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
        let scheme = if self.tls {
            crate::SCHEME_HTTPS
        } else {
            crate::SCHEME_HTTP
        };
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
        Self {
            prefix: "web:log".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub listen: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
    pub default_host: HostConfig,
    pub hosts: Vec<HostConfig>,
    pub authorities: Option<Vec<String>>,

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
            authorities: None,
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
    pub fn new_host(
        host: HostConfig,
        port: u16,
        tls: Option<TlsConfig>,
    ) -> Self {
        Self {
            listen: String::from(crate::config::ADDR),
            port: port,
            tls,
            redirect_insecure: true,
            temporary_redirect: true,
            default_host: host,
            authorities: None,
            hosts: vec![],
        }
    }

    pub fn hosts(&self) -> Vec<HostConfig> {
        let mut list = vec![self.default_host.clone()];
        let mut hosts =
            self.hosts.iter().map(|h| h.clone()).collect::<Vec<_>>();
        list.append(&mut hosts);
        list
    }

    pub fn authorities(&self) -> &Option<Vec<String>> {
        &self.authorities
    }

    pub fn get_port(&self, port_type: PortType) -> u16 {
        get_port(self.port, &self.tls, port_type)
    }

    pub fn tls_port(&self) -> u16 {
        if let Some(ref tls) = self.tls {
            tls.port
        } else {
            crate::PORT_SSL
        }
    }

    pub fn get_address(
        &self,
        port_type: PortType,
        host: Option<&str>,
    ) -> String {
        let port = self.get_port(port_type);
        let host = if let Some(host) = host {
            host.clone()
        } else {
            &self.listen
        };
        format!("{}:{}", host, port)
    }

    pub fn get_url(
        &self,
        scheme: &str,
        port_type: PortType,
        host: Option<&str>,
    ) -> String {
        format!("{}//{}", scheme, self.get_address(port_type, host))
    }

    pub fn get_host_url(&self, host: &str) -> String {
        let scheme = if self.tls.is_some() {
            crate::SCHEME_HTTPS
        } else {
            crate::SCHEME_HTTP
        };
        format!(
            "{}//{}",
            scheme,
            self.get_address(PortType::Infer, Some(host))
        )
    }

    pub fn get_sock_addr(&self, port_type: PortType) -> Result<SocketAddr> {
        let address = self.get_address(port_type, None);
        Ok(address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::NoSocketAddress(address))?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostConfig {
    /// Host name.
    pub name: String,

    /// Directory for static files.
    pub directory: PathBuf,

    /// Configuration for webdav.
    pub webdav: Option<WebDavConfig>,

    /// Directory for the editor UI static files.
    pub editor_directory: Option<PathBuf>,

    #[serde(skip)]
    pub redirects: Option<Redirects>,

    /// Websocket endpoint when watching for file system changes.
    #[serde(skip)]
    pub endpoint: Option<String>,
    /// Send headers that instruct browsers to disable caching.
    #[serde(skip)]
    pub disable_cache: bool,
    #[serde(skip)]
    pub deny_iframe: bool,

    /// Log server requests.
    #[serde(skip)]
    pub log: bool,
    /// Flag that indicates this host should be configured
    /// for file system watching.
    #[serde(skip)]
    pub watch: bool,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            name: crate::config::HOST.to_string(),
            directory: PathBuf::from(""),
            webdav: None,
            editor_directory: None,
            redirects: None,
            endpoint: None,
            disable_cache: false,
            deny_iframe: true,
            log: false,
            watch: false,
        }
    }
}

impl HostConfig {
    pub fn new(
        directory: PathBuf,
        name: String,
        redirects: Option<Redirects>,
        endpoint: Option<String>,
        log: bool,
        watch: bool,
    ) -> Self {
        Self {
            directory,
            name,
            redirects,
            endpoint,
            disable_cache: true,
            deny_iframe: true,
            log,
            watch,
            webdav: None,
            editor_directory: None,
        }
    }


    /// Attempt to load from a redirects file into this host.
    pub fn load_redirects(&mut self) -> Result<()> {
        let redirects = self.directory.join(crate::REDIRECTS_FILE);
        if redirects.exists() {
            let contents = fs::read_to_string(&redirects)?;
            self.redirects = Some(serde_json::from_str(&contents)?);
        }
        Ok(())
    }

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebDavConfig {
    /// Directory for the webdav mount point.
    pub directory: PathBuf,
    /// Whether to list directories.
    pub listing: bool,
}
