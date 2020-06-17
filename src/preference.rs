use std::fs;
use std::path::PathBuf;

use home;
use serde_with::skip_serializing_none;
use serde::{Deserialize, Serialize};

use crate::Error;

static ROOT_DIR: &str = ".hypertext";
static PREFERENCES: &str = "preferences.toml";

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
            lang: Some(String::from("en")),
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

pub fn init() -> Result<(), Error> {
    let mut buf = get_root_dir()?;
    buf.push(PREFERENCES);

    if !buf.exists() {
    
    } else {
        return Err(
            Error::new(
                format!("Preferences file '{}' exists, please move it away", buf.display())))
    }

    Ok(())
}
