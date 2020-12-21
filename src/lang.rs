use std::fs::{self, File};
use std::path::PathBuf;

use log::{info, warn};

use crate::{Error, Result, opts::{self, Lang}};
use config::{Config, ProfileSettings};
use locale::Locales;
use unic_langid::LanguageIdentifier;


//use super::Lang;
//use uwe::{lang, opts, Result};


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
async fn list(project: PathBuf) -> Result<()> {
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    let workspace = workspace::open(&project, true)?;
    for entry in workspace.into_iter() {
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

/// Add languages to a project.
///
/// If the locales directory and common messsages file do not exist
/// they are created as is the messages file for the fallback language.
async fn new(project: PathBuf, languages: Vec<String>) -> Result<()> {
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    // Ensure we have valid language identifiers
    for lang in languages.iter() {
        let _: LanguageIdentifier = lang.parse()?;
    }

    let workspace = workspace::open(&project, true)?;
    for entry in workspace.into_iter() {
        let info = LanguageInfo::new(entry.config)?;
        info!(
            "Project {} (language: {})",
            info.config.project().display(),
            info.config.lang
        );

        fs::create_dir_all(&info.locales_dir)?;

        let mut files = vec![
            PathBuf::from(config::CORE_FTL),
            PathBuf::from(&info.config.lang).join(config::MAIN_FTL),
        ];

        // Filter out the primary fallback language if it was specified
        let langs: Vec<&String> = languages
            .iter()
            .filter(|&s| s != &info.config.lang)
            .collect();

        for lang in langs.iter() {
            files.push(PathBuf::from(lang).join(config::MAIN_FTL));
        }

        for f in files.iter() {
            let target = info.locales_dir.join(f);
            if !target.exists() {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(&parent)?;
                }
                info!("Create {}", target.display());
                let _ = File::create(target)?;
            } else {
                warn!("File {} exists, skip creation", target.display());
            }
        }
    }

    Ok(())
}

pub async fn run(cmd: Lang) -> Result<()> {
    match cmd {
        Lang::List { project } => {
            let project = opts::project_path(&project)?;
            list(project).await?;
        }

        Lang::New { project, languages } => {
            let project = opts::project_path(&project)?;
            new(project, languages).await?;
        }
    }

    Ok(())
}
