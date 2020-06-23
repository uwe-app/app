use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

use log::{info, warn};
use serde::{Deserialize, Serialize};
use home;

use crate::Result;
use crate::utils;
use crate::cache::{self, CacheComponent};
use crate::preference;
use crate::utils::symlink;

static BASH: &str = "bash";
static ZSH: &str = "zsh";

static NAME: &str = "ht";
static VERSION_FILE: &str = "version.toml";

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct VersionInfo {
    pub version: String,
}

pub fn version() -> Result<(PathBuf, VersionInfo)> {
    let mut version_file = cache::get_release_dir()?;
    version_file.push(VERSION_FILE);
    let content = utils::read_string(&version_file)?;
    let info: VersionInfo = toml::from_str(&content)?;
    Ok((version_file, info))
}

#[cfg(unix)]
pub fn get_source_env() -> String {
    format!("source $HOME/.hypertext/env\n")
}

#[cfg(windows)]
pub fn get_source_env() -> String {
    println!("TODO: handle source env file for windows");
    format!("source $HOME/.hypertext/env\n")
}

// TODO: switch this for windows too!
pub fn get_env_content(bin_dir: &PathBuf) -> String {
    format!("export PATH=\"{}:$PATH\"\n", bin_dir.display())
}

// Write out the env file
pub fn write_env(bin_dir: &PathBuf) -> Result<()> {
    let content = get_env_content(bin_dir);
    let env = cache::get_env_file()?;
    utils::write_string(env, content)?;
    Ok(())
}

pub fn source_env(_bin_dir: &PathBuf) -> Result<(bool, bool, String, PathBuf)> {
    let mut files: HashMap<String, Vec<String>> = HashMap::new();
    files.insert(BASH.to_string(), vec![".bashrc".to_string()]);
    files.insert(ZSH.to_string(), vec![".zshrc".to_string()]);

    if let Some(home_dir) = home::home_dir() {
        let source_path = get_source_env();
        if let Ok(shell) = std::env::var("SHELL") {
            let shell_path = PathBuf::from(shell);
            if let Some(name) = shell_path.file_name() {
                let name = name.to_string_lossy().into_owned();
                if let Some(entries) = files.get(&name) {
                    for f in entries {
                        let mut file = home_dir.clone();
                        file.push(f);
                        let mut contents = utils::read_string(&file)?;
                        if !contents.contains(&source_path) {
                            contents.push_str(&source_path);
                            utils::write_string(&file, contents)?;
                            return Ok((true, true, name.to_string(), file));
                        }
                        return Ok((true, false, name.to_string(), file));
                    }
                }
            }
        }
    }

    Ok((false, false, String::from(""), PathBuf::from("")))
}

pub fn install() -> Result<()> {
    match update() {
        Ok((name, info, bin, bin_dir)) => {
            // Write out the env file
            write_env(&bin_dir)?;

            // Try to configure the shell paths
            let (shell_ok, shell_write, shell_name, shell_file) = source_env(&bin_dir)?;
            if shell_ok {
                if shell_write {
                    info!("");
                    info!("Updated {} at {}", shell_name, shell_file.display());
                }
            } else {
                warn!(""); 
                warn!("Update your PATH to include {}", bin_dir.display());
            }

            let source_path = get_source_env().trim().to_string();

            info!("");
            info!("Installation complete!");
            info!("");
            info!("To update your current shell session run:");
            info!("");
            info!("   {}", source_path);
            info!("");

            info!("Installed {}@{} to {}", name, info.version, bin.display());
        },
        Err(e) => return Err(e),
    }
    Ok(())
}

pub fn update() -> Result<(String, VersionInfo, PathBuf, PathBuf)>  {
    let prefs = preference::load()?;
    let components = vec![CacheComponent::Release];
    cache::update(&prefs, components)?;

    let (version_file, info) = version()?;

    let bin_dir = cache::get_bin_dir()?;
    let mut bin = bin_dir.clone();
    let mut release_bin = cache::get_release_bin_dir()?;
    bin.push(NAME);
    release_bin.push(NAME);

    if bin.exists() {
        std::fs::remove_file(&bin)?;
    }

    symlink::soft(&release_bin, &bin)?; 

    // Copy the version file so we know which version
    // was installed the last time that update() was run
    let mut dest_version = cache::get_root_dir()?;
    dest_version.push(VERSION_FILE);
    fs::copy(version_file, dest_version)?;

    Ok((NAME.to_string(), info, bin, bin_dir))
}
