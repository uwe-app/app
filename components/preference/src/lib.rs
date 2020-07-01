use std::io;
use std::path::PathBuf;

use thiserror::Error;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use dirs;
use utils;

static PREFERENCES: &str = "preferences.toml";
static LANG: &str = "en";
static DEFAULT_BLUEPRINT_PATH: &str = "style/normalize";

pub static BLUEPRINT_URL: &str = "https://github.com/hypertext-live/blueprint";

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Preferences {
    pub lang: Option<String>,
    pub blueprint: Option<BlueprintPreferences>,
    pub ssh: Option<SshPreferences>,
    pub docs: Option<DocsPreferences>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            lang: Some(String::from(LANG)),
            ssh: None,
            blueprint: Some(Default::default()),
            docs: Some(Default::default()),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct BlueprintPreferences {
    pub url: Option<String>,
    pub default_path: Option<String>,
}

impl Default for BlueprintPreferences {
    fn default() -> Self {
        Self {
            url: Some(String::from(BLUEPRINT_URL)),
            default_path: Some(String::from(DEFAULT_BLUEPRINT_PATH)),
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

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocsPreferences {
    pub host: String,
    pub port: u16,
}

impl Default for DocsPreferences {
    fn default() -> Self {
        Self {
            host: String::from("localhost"),
            port: 0,
        }
    }
}

pub fn get_prefs_file() -> io::Result<PathBuf> {
    let mut buf = dirs::get_root_dir()?;
    buf.push(PREFERENCES);
    Ok(buf)
}

pub fn load_file() -> io::Result<String> {
    let buf = get_prefs_file()?;
    utils::fs::read_string(&buf)
}

pub fn load() -> Result<Preferences> {
    let buf = get_prefs_file()?;
    let mut prefs: Preferences = Default::default();
    if buf.exists() {
        let content = load_file()?;
        prefs = toml::from_str(&content)?;
    }
    Ok(prefs)
}

pub fn init_if_none() -> Result<()> {
    let buf = get_prefs_file()?;
    if !buf.exists() {
        let prefs: Preferences = Default::default();
        let content = toml::to_string(&prefs)?;
        utils::fs::write_string(buf, content)?;
    }
    Ok(())
}

