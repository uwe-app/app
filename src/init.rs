use std::fs;
use std::path::Path;
use std::path::PathBuf;

use toml::map::Map;
use toml::value::Value;

use preference::{self, Preferences};
use utils::walk;

use crate::Error;

#[derive(Debug)]
pub struct InitOptions {
    pub target: PathBuf,
    pub source: Option<String>,
    pub language: Option<String>,
    pub host: Option<String>,
    pub locales: Option<String>,
}

// Read, modify and write the site configuration
// with options for language, host and locales.
fn write_settings<P: AsRef<Path>>(
    output: P,
    prefs: &Preferences,
    lang: Option<String>,
    host: Option<String>,
    locale_ids: Vec<String>,
) -> Result<(), Error> {
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

pub fn init(options: InitOptions) -> Result<(), Error> {
    let prefs = preference::load()?;
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

    let target = options.target;
    let message = "Initial files.";

    if target.exists() {
        return Err(Error::TargetExists(target.clone()));
    }

    // Clone an existing blueprint
    if let Some(ref source) = options.source {
        if let Some(ref parent) = target.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let repo = git::clone(source, &target)?;

        let site_toml = target.join(config::SITE_TOML);
        if !site_toml.exists() {
            return Err(Error::NoSiteSettings(
                target, config::SITE_TOML.to_string()));
        }

        write_settings(&target, &prefs, language, options.host, locale_ids)?;
        git::pristine(&target, repo, message)?;
    } else {
        let source = cache::get_default_blueprint()?;
        walk::copy(&source, &target, |_| true)?;
        write_settings(&target, &prefs, language, options.host, locale_ids)?;
        git::init(&target, message)?;
    };

    Ok(())
}
