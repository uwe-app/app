use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use thiserror::Error;

use dirs;
use utils;

static PREFERENCES: &str = "preferences.toml";
static LANG: &str = "en";

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Preferences {
    pub lang: Option<String>,
    pub ssh: Option<SshPreferences>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            lang: Some(String::from(LANG)),
            ssh: None,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct SshPreferences {
    pub default_key: Option<PathBuf>,
}

impl Default for SshPreferences {
    fn default() -> Self {
        Self { default_key: None }
    }
}

pub fn get_prefs_file() -> io::Result<PathBuf> {
    let mut buf = dirs::root_dir()?;
    buf.push(PREFERENCES);
    Ok(buf)
}

pub fn load_file() -> io::Result<String> {
    let buf = get_prefs_file()?;
    utils::fs::read_string(&buf)
}

pub fn load() -> Result<Preferences, Error> {
    let buf = get_prefs_file()?;
    let mut prefs: Preferences = Default::default();
    if buf.exists() {
        let content = load_file()?;
        prefs = toml::from_str(&content)?;
    }
    Ok(prefs)
}

pub fn init_if_none() -> Result<(), Error> {
    let buf = get_prefs_file()?;
    if !buf.exists() {
        let prefs: Preferences = Default::default();
        let content = toml::to_string(&prefs)?;
        utils::fs::write_string(buf, content)?;
    }
    Ok(())
}
