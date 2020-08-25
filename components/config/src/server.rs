use std::collections::HashMap;
use std::path::PathBuf;

use http::Uri;

use serde::{Deserialize, Serialize};

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
}

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
