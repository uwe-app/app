use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::info;
use toml::map::Map;
use toml::value::Value;
use url::Url;

use utils::walk;

use crate::{Error, Result};

static DEFAULT_NAME: &str = "default";
static DEFAULT_MESSAGE: &str = "Initial files.";

#[derive(Debug)]
pub struct ProjectOptions {
    pub target: PathBuf,
    pub message: Option<String>,
    pub source: Option<String>,
    pub language: Option<String>,
    pub host: Option<String>,
    pub locales: Option<String>,
    pub remote_name: String,
    pub remote_url: Option<String>,
}

struct InitSettings {
    language: Option<String>,
    host: Option<String>,
    locale_ids: Vec<String>,
}

// Read, modify and write the site configuration
// with options for language, host and locales.
fn write_settings<P: AsRef<Path>>(
    output: P,
    settings: InitSettings,
) -> Result<()> {
    let prefs = preference::load()?;
    let lang = settings.language;
    let host = settings.host;
    let locale_ids = settings.locale_ids;

    // This is used later to determine whether a redirect should be created
    let has_custom_lang = lang.is_some() || !locale_ids.is_empty();

    // If we were passed a specific language use it
    let language = if lang.is_some() {
        lang
    // Otherwise if we have locales prefer the first in the list
    } else {
        if !locale_ids.is_empty() {
            Some(locale_ids[0].clone())
        } else {
            prefs.lang.clone()
        }
    };

    let target = output.as_ref().to_path_buf();
    let mut config_file = target.clone();
    config_file.push(config::SITE_TOML);
    let mut site_config: toml::value::Table =
        toml::from_str(&utils::fs::read_string(&config_file)?)?;
    if let Some(ref lang) = language {
        site_config.insert(
            config::LANG_KEY.to_string(),
            Value::String(lang.to_string()),
        );
    }
    if let Some(host) = host {
        site_config.insert(config::HOST_KEY.to_string(), Value::String(host));
    }

    let empty = String::from("");

    if !locale_ids.is_empty() {
        let mut site_dir = target.clone();
        site_dir.push(config::SITE);

        let mut locales_dir = site_dir.clone();
        locales_dir.push(config::LOCALES);

        let mut core_file = locales_dir.clone();
        core_file.push(config::CORE_FTL);
        utils::fs::write_string(&core_file, &empty)?;

        for id in locale_ids {
            let mut lang_file = locales_dir.clone();
            lang_file.push(id);
            lang_file.push(config::MAIN_FTL);
            utils::fs::write_string(&lang_file, &empty)?;
        }
    }

    let mut fluent: Map<String, Value> = Map::new();
    if let Some(ref lang) = language {
        fluent.insert(
            config::FALLBACK_KEY.to_string(),
            Value::String(lang.to_string()),
        );
    }
    fluent.insert(
        config::SHARED_KEY.to_string(),
        Value::String(config::CORE_FTL.to_string()),
    );
    site_config.insert(config::FLUENT_KEY.to_string(), Value::Table(fluent));

    let mut redirect: Map<String, Value> = Map::new();
    if has_custom_lang {
        if let Some(ref lang) = language {
            redirect
                .insert("/".to_string(), Value::String(format!("/{}/", lang)));
            site_config.insert(
                config::REDIRECT_KEY.to_string(),
                Value::Table(redirect),
            );
        }
    }

    utils::fs::write_string(&config_file, toml::to_string(&site_config)?)?;

    Ok(())
}

/// Initialize a project copying files from a source folder.
fn init_folder<S: AsRef<Path>, T: AsRef<Path>>(
    source: S,
    target: T,
    settings: InitSettings,
    message: &str,
) -> Result<()> {
    create_target_parents(target.as_ref())?;
    walk::copy(source.as_ref(), target.as_ref(), |_| true)?;
    write_settings(target.as_ref(), settings)?;
    scm::init(target.as_ref(), message)?;

    Ok(())
}

/// Check a folder has the site settings configuration file.
fn check_site_settings<T: AsRef<Path>>(target: T) -> Result<()> {
    let site_toml = target.as_ref().join(config::SITE_TOML);
    if !site_toml.exists() || !site_toml.is_file() {
        return Err(Error::NoSiteSettings(
            target.as_ref().to_path_buf(),
            config::SITE_TOML.to_string(),
        ));
    }

    Ok(())
}

/// Create parent directories for the target project.
fn create_target_parents<T: AsRef<Path>>(target: T) -> Result<()> {
    if let Some(ref parent) = target.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

pub fn project(options: ProjectOptions) -> Result<()> {
    let mut language = None;

    if let Some(ref lang) = options.language {
        config::parse_language(lang)?;
        language = options.language.clone();
    }

    if let Some(ref host) = options.host {
        config::parse_host(host)?;
    }

    let mut locale_ids = Vec::new();
    if let Some(ref locales) = options.locales {
        let locale_list = locales
            .split(",")
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();
        for lang_id in locale_list {
            config::parse_language(&lang_id)?;
            locale_ids.push(lang_id);
        }
    }

    // This should prevent inadvertently creating a redirect
    // to a non-existent locale if the given language does
    // not exist in the list of locales
    if !locale_ids.is_empty() {
        if let Some(ref lang) = language {
            if !locale_ids.contains(lang) {
                return Err(Error::LanguageMissingFromLocales(
                    lang.clone(),
                    locale_ids.join(","),
                ));
            }
        }
    }

    let settings = InitSettings {
        language,
        host: options.host,
        locale_ids,
    };

    let target = options.target;
    let message: &str = if let Some(ref message) = options.message {
        message
    } else {
        DEFAULT_MESSAGE
    };

    if target.exists() {
        return Err(Error::TargetExists(target.clone()));
    }

    // 1) Check for URL; if valid clone as a scm repository
    // 2) Check for local file system folder; if valid copy files.
    // 3) Check for named blueprint folder; if valid copy files.
    if let Some(ref source) = options.source {
        match source.parse::<Url>() {
            Ok(url) => {
                create_target_parents(&target)?;
                scm::copy(url.to_string(), &target, message)?;
                check_site_settings(&target)?;
                write_settings(&target, settings)?;
            }
            Err(_) => {
                let source_dir = PathBuf::from(source);
                let is_local_folder =
                    source_dir.exists() && source_dir.is_dir();
                let source_dir = if is_local_folder {
                    source_dir
                } else {
                    let blueprints = dirs::blueprints_dir()?;
                    blueprints.join(source)
                };

                if !source_dir.exists() {
                    return Err(Error::NoInitSource);
                }

                check_site_settings(&source_dir)?;
                init_folder(source_dir, &target, settings, message)?;
            }
        }

    // 4) No source specified so we just use the default blueprint.
    } else {
        let source = dirs::blueprints_dir()?.join(DEFAULT_NAME);
        check_site_settings(&source)?;
        init_folder(source, &target, settings, message)?;
    };

    if let Some(ref url) = options.remote_url {
        scm::set_remote(&target, &options.remote_name, url)?;
    }

    info!("Created {} ✓", target.to_string_lossy());

    Ok(())
}
