use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
}

