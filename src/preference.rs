use std::fs;
use std::path::PathBuf;

use home;
use serde_with::skip_serializing_none;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::utils;

static ROOT_DIR: &str = ".hypertext";
static PREFERENCES: &str = "preferences.toml";
static LANG: &str = "en";

// FIXME: use a different framework agnostic default
static DEFAULT_BLUEPRINT_PATH: &str = "vanilla/newcss";

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Preferences {
    pub lang: Option<String>,
    pub blueprint: Option<BlueprintPreferences>,
    pub ssh: Option<SshPreferences>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            lang: Some(String::from(LANG)),
            ssh: None,
            blueprint: Some(Default::default()),
        } 
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct BlueprintPreferences {
    pub default_path: Option<String>,
}

impl Default for BlueprintPreferences {
    fn default() -> Self {
        Self {
            default_path: Some(String::from(DEFAULT_BLUEPRINT_PATH))
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
        Self {default_key: None}
    }
}

pub fn get_root_dir() -> Result<PathBuf, Error> {
    let cache = home::home_dir();
    if let Some(ref cache) = cache {
        let mut buf = cache.clone();
        buf.push(ROOT_DIR);
        if !buf.exists() {
            fs::create_dir_all(&buf)?;
        }
        return Ok(buf);
    }
    Err(
        Error::new(
            format!("Could not determine home directory")))
}

pub fn get_prefs_file() -> Result<PathBuf, Error> {
    let mut buf = get_root_dir()?;
    buf.push(PREFERENCES);
    Ok(buf)
}

pub fn load() -> Result<Preferences, Error> {
    let buf = get_prefs_file()?;
    let mut prefs: Preferences = Default::default();
    if buf.exists() {
        let content = utils::read_string(&buf)?;
        prefs = toml::from_str(&content)?;
    }
    Ok(prefs)
}

pub fn init() -> Result<(), Error> {
    let buf = get_prefs_file()?;
    if !buf.exists() {
        let prefs: Preferences = Default::default();
        let content = toml::to_string(&prefs)?; 
        utils::write_string(buf, content)?;
    } else {
        return Err(
            Error::new(
                format!(
                    "Preferences file '{}' exists, please move it away", buf.display())))
    }

    Ok(())
}
