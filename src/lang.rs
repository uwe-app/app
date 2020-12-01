use std::path::PathBuf;

use log::info;

use crate::{Error, Result};
use config::{Config, ProfileSettings};
use locale::{LocaleMap, Locales};

struct LanguageInfo {
    config: Config,
    locales: Locales,
    locales_dir: PathBuf,
    has_locales: bool,
}

impl LanguageInfo {
    pub fn new(config: Config) -> Result<Self> {
        let profile = config.build.as_ref().unwrap();
        let args: ProfileSettings = Default::default();
        let project = config.project();
        let source = project.join(&args.source);
        let locales_dir = source.join(profile.locales.as_ref().unwrap());
        let mut locales: Locales = Default::default();
        let has_locales = locales_dir.exists() && locales_dir.is_dir();
        if has_locales {
            let _ = locales.load(&config, &locales_dir)?;
        }
        Ok(Self {
            config,
            locales,
            locales_dir,
            has_locales,
        })
    }
}

/// List languages for a project.
pub async fn list(project: PathBuf) -> Result<()> {
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    let workspace = workspace::open(&project, true)?;
    for mut entry in workspace.into_iter() {
        let info = LanguageInfo::new(entry.config)?;
        info!(
            "Project {} (language: {})",
            info.config.project().display(),
            info.config.lang
        );
        if info.has_locales {
            info!("Locales {}", info.locales_dir.display());
            let map = info.locales.languages();
            for id in map.alternate() {
                info!("Translation {}", id);
            }
        }
    }

    Ok(())
}

/// Create the locales directory and files for the fallback language.
pub async fn init(project: PathBuf) -> Result<()> {
    Ok(())
}

/// Add a language to a project.
pub async fn add(project: PathBuf) -> Result<()> {
    Ok(())
}
