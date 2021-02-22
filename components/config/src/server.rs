use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result, redirect::Redirects, memfs::EmbeddedFileSystem};

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
    tls: &Option<SslConfig>,
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
pub struct SslConfig {
    cert: PathBuf,
    key: PathBuf,
    port: u16,
}

impl SslConfig {
    pub fn new(cert: PathBuf, key: PathBuf, port: u16) -> Self {
        Self { cert, key, port }
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn cert(&self) -> &PathBuf {
        &self.cert
    }

    pub fn key(&self) -> &PathBuf {
        &self.key
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub open: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ServerConfig {
    listen: String,

    port: u16,

    #[serde(default = "num_cpus::get")]
    workers: usize,

    ssl: Option<SslConfig>,

    #[serde(default, alias = "host")]
    hosts: Vec<HostConfig>,

    #[serde(default)]
    authorities: Option<Vec<String>>,

    /// When running a server over SSL redirect HTTP to HTTPS.
    #[serde(skip)]
    redirect_insecure: bool,

    /// Whether redirects should use a temporary status code.
    #[serde(skip)]
    temporary_redirect: bool,

    #[serde(skip)]
    allow_ssl_from_env: bool,

    #[serde(skip)]
    disable_signals: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: String::from(crate::config::HOST),
            port: crate::config::PORT,
            workers: num_cpus::get(),
            ssl: None,
            redirect_insecure: true,
            temporary_redirect: false,
            allow_ssl_from_env: true,
            authorities: None,
            hosts: vec![],
            disable_signals: false,
        }
    }
}

impl ServerConfig {
    /// New configuration for a host and port.
    pub fn new(listen: String, port: u16, ssl: Option<SslConfig>) -> Self {
        let mut tmp: Self = Default::default();
        tmp.listen = listen;
        tmp.port = port;
        tmp.ssl = ssl;
        tmp
    }

    pub fn load<P: AsRef<Path>>(file: P) -> Result<ServerConfig> {
        let contents = fs::read_to_string(file.as_ref())?;
        let mut config: ServerConfig = toml::from_str(&contents)?;

        // Directory paths that are relative should be resolved
        // using the parent folder of the configuration file.
        if let Some(parent) = file.as_ref().parent() {
            for host in config.hosts.iter_mut() {
                let dir = host.directory().to_string_lossy();
                if !dir.is_empty() && host.directory().is_relative() {
                    host.directory = parent.join(host.directory());
                }
            }
        }

        Ok(config)
    }

    pub fn compute_ssl(&self) -> Option<SslConfig> {
        let (ssl_cert, ssl_key, ssl_port) = if let Some(ref ssl) = self.ssl {
            (
                Some(ssl.cert.to_path_buf()),
                Some(ssl.key.to_path_buf()),
                ssl.port(),
            )
        } else {
            if self.allow_ssl_from_env {
                (
                    std::env::var("UWE_SSL_CERT").ok().map(PathBuf::from),
                    std::env::var("UWE_SSL_KEY").ok().map(PathBuf::from),
                    crate::PORT_SSL,
                )
            } else {
                (None, None, crate::PORT_SSL)
            }
        };

        if let (Some(cert), Some(key)) = (ssl_cert.as_ref(), ssl_key.as_ref()) {
            // Allow empty environment variables as a means of disabling SSL certificates
            let is_cert_empty = cert.to_string_lossy().is_empty();
            let is_key_empty = key.to_string_lossy().is_empty();
            if !is_cert_empty && !is_key_empty {
                Some(SslConfig::new(
                    cert.to_path_buf(),
                    key.to_path_buf(),
                    ssl_port,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn ssl(&self) -> &Option<SslConfig> {
        &self.ssl
    }

    pub fn ssl_mut(&mut self) -> &mut Option<SslConfig> {
        &mut self.ssl
    }

    pub fn has_ssl(&self) -> bool {
        self.ssl.is_some()
    }

    pub fn workers(&self) -> usize {
        self.workers
    }

    pub fn add_host(&mut self, host: HostConfig) {
        self.hosts.push(host);
    }

    pub fn set_hosts(&mut self, hosts: Vec<HostConfig>) {
        self.hosts = hosts;
    }

    pub fn hosts(&self) -> &Vec<HostConfig> {
        &self.hosts
    }

    pub fn set_authorities(&mut self, authorities: Option<Vec<String>>) {
        self.authorities = authorities
    }

    pub fn authorities(&self) -> &Option<Vec<String>> {
        &self.authorities
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn get_port(&self, port_type: PortType) -> u16 {
        get_port(self.port, &self.ssl, port_type)
    }

    pub fn ssl_port(&self) -> u16 {
        if let Some(ref ssl) = self.ssl {
            ssl.port
        } else {
            crate::PORT_SSL
        }
    }

    pub fn set_allow_ssl_from_env(&mut self, flag: bool) {
        self.allow_ssl_from_env = flag;
    }

    pub fn disable_signals(&self) -> bool {
        self.disable_signals
    }

    pub fn set_disable_signals(&mut self, flag: bool) {
        self.disable_signals = flag;
    }

    pub fn redirect_insecure(&self) -> bool {
        self.redirect_insecure
    }

    pub fn set_redirect_insecure(&mut self, flag: bool) {
        self.redirect_insecure = flag;
    }

    pub fn temporary_redirect(&self) -> bool {
        self.temporary_redirect
    }

    pub fn set_temporary_redirect(&mut self, flag: bool) {
        self.temporary_redirect = flag;
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
        let scheme = if self.ssl.is_some() {
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
#[serde(default, rename_all = "kebab-case")]
pub struct HostConfig {
    /// Host name.
    name: String,

    /// Directory for static files.
    directory: PathBuf,

    /// Embedded file system, overrides any directory
    /// setting.
    #[serde(skip)]
    embedded: Option<Box<dyn EmbeddedFileSystem>>,

    /// Require an index page inside the directory.
    require_index: bool,

    /// Send headers that instruct browsers to disable caching.
    disable_cache: bool,

    /// Deny embedding as an iframe.
    deny_iframe: bool,

    /// Log server requests.
    log: bool,

    /// Configuration for webdav.
    #[serde(skip)]
    webdav: Option<WebDavConfig>,

    #[serde(skip)]
    redirects: Option<Redirects>,

    /// Websocket endpoint when watching for file system changes.
    #[serde(skip)]
    endpoint: Option<String>,

    /// Flag that indicates this host should be configured
    /// for file system watching.
    #[serde(skip)]
    watch: bool,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            name: crate::config::HOST.to_string(),
            directory: PathBuf::from(""),
            embedded: None,
            webdav: None,
            redirects: None,
            endpoint: None,
            disable_cache: false,
            require_index: true,
            deny_iframe: true,
            log: false,
            watch: false,
        }
    }
}

impl HostConfig {
    pub fn new(name: String, directory: PathBuf) -> Self {
        let mut host: HostConfig = Default::default();
        host.name = name;
        host.directory = directory;
        host
    }

    pub fn new_directory(directory: PathBuf) -> Self {
        let mut host: HostConfig = Default::default();
        host.directory = directory;
        host
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    pub fn set_directory(&mut self, directory: PathBuf) {
        self.directory = directory;
    }

    pub fn embedded(&self) -> &Option<Box<dyn EmbeddedFileSystem>> {
        &self.embedded
    }

    pub fn set_embedded(&mut self, fs: Option<Box<dyn EmbeddedFileSystem>>) {
        self.embedded = fs;
    }

    pub fn require_index(&self) -> bool {
        self.require_index
    }

    pub fn set_require_index(&mut self, require_index: bool) {
        self.require_index = require_index;
    }

    pub fn disable_cache(&self) -> bool {
        self.disable_cache
    }

    pub fn set_disable_cache(&mut self, disable_cache: bool) {
        self.disable_cache = disable_cache;
    }

    pub fn deny_iframe(&self) -> bool {
        self.deny_iframe
    }

    pub fn endpoint(&self) -> &Option<String> {
        &self.endpoint
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        self.endpoint = Some(endpoint);
    }

    pub fn watch(&self) -> bool {
        self.watch
    }

    pub fn set_watch(&mut self, watch: bool) {
        self.watch = watch;
    }

    pub fn log(&self) -> bool {
        self.log
    }

    pub fn set_log(&mut self, log: bool) {
        self.log = log;
    }

    pub fn redirects(&self) -> &Option<Redirects> {
        &self.redirects
    }

    pub fn set_redirects(&mut self, redirects: Option<Redirects>) {
        self.redirects = redirects;
    }

    pub fn webdav(&self) -> &Option<WebDavConfig> {
        &self.webdav
    }

    pub fn set_webdav(&mut self, webdav: Option<WebDavConfig>) {
        self.webdav = webdav;
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
    /// Path in the virtual host used to mount the webdav directory.
    mount_path: String,
    /// Directory for the webdav mount point.
    directory: PathBuf,
    /// Whether to list directories.
    listing: bool,
}

impl WebDavConfig {
    pub fn new(mount_path: String, directory: PathBuf, listing: bool) -> Self {
        Self {
            mount_path,
            directory,
            listing,
        }
    }

    pub fn mount_path(&self) -> &str {
        &self.mount_path
    }

    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    pub fn listing(&self) -> bool {
        self.listing
    }
}
