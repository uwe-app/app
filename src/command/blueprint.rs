use std::fs;
use std::path::Path;
use std::path::PathBuf;

use toml::map::Map;
use toml::value::Value;

use preference::{self, Preferences};

use crate::Error;

#[derive(Debug)]
pub struct InitOptions {
    pub source: Option<String>,
    pub target: Option<PathBuf>,
    pub private_key: Option<PathBuf>,

    pub language: Option<String>,
    pub host: Option<String>,
    pub locales: Option<String>,
}

// Read, modify and write the site configuration
// with options for language, host and multilingual locales
fn write_options<P: AsRef<Path>>(
    output: P,
    prefs: &Preferences,
    lang: Option<String>,
    host: Option<String>,
    locale_ids: Vec<String>) -> Result<(), Error> {

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
    let mut site_config: toml::value::Table = toml::from_str(&utils::fs::read_string(&config_file)?)?;
    if let Some(ref lang) = language {
        site_config.insert(config::LANG_KEY.to_string(), Value::String(lang.to_string()));
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
        fluent.insert(config::FALLBACK_KEY.to_string(), Value::String(lang.to_string()));
    }
    fluent.insert(config::SHARED_KEY.to_string(), Value::String(config::CORE_FTL.to_string()));
    site_config.insert(config::FLUENT_KEY.to_string(), Value::Table(fluent));

    let mut redirect: Map<String, Value> = Map::new();
    if let Some(ref lang) = language {
        redirect.insert("/".to_string(), Value::String(format!("/{}/", lang)));
        site_config.insert(config::REDIRECT_KEY.to_string(), Value::Table(redirect));
    }

    utils::fs::write_string(&config_file, toml::to_string(&site_config)?)?;
    Ok(())
}

fn prepare() -> Result<(Preferences, String, PathBuf), Error> {
    let prefs = preference::load()?;
    let url = cache::get_blueprint_url(&prefs);
    let blueprint_cache_dir = cache::get_blueprint_dir()?;
    if !blueprint_cache_dir.exists() {
        git::print_clone(&url, &blueprint_cache_dir);
    }
    Ok((prefs, url, blueprint_cache_dir))
}

pub fn list() -> Result<(), Error> {
    let (_, url, blueprint_cache_dir) = prepare()?;
    let (repo, _cloned) = git::open_or_clone(&url, &blueprint_cache_dir, true)?;
    git::list_submodules(repo)?;
    Ok(())
}

pub fn init(options: InitOptions) -> Result<(), Error> {
    let (prefs, _url, _blueprint_cache_dir) = prepare()?;

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
                return Err(
                    Error::new(
                        format!(
                            "Language '{}' does not exist in the locales '{}'", lang, locale_ids.join(","))));
            }
        }
    }

    if let Some(ref target) = options.target {
        if target.exists() {
            return Err(Error::new(format!(
                "Target '{}' exists, please move it away",
                target.display()
            )));
        }

        let repo;
        let repo_url = cache::get_blueprint_url(&prefs);
        let repo_dir = cache::get_blueprint_dir()?;

        let source = if let Some(ref source) = options.source {
            source.clone()
        } else {
            if let Some(ref source) = prefs.blueprint.as_ref().unwrap().default_path {
                source.clone()
            } else {
                "".to_string()
            }
        };

        if source.is_empty() {
            return Err(Error::new(format!(
                "Could not determine default source path"
            )));
        }

        if let Some(ref parent) = target.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
            repo = git::create(
                source,
                target,
                options.private_key.clone(),
                repo_url,
                repo_dir,
            )?;
        } else {
            repo = git::create(
                source,
                target,
                options.private_key.clone(),
                repo_url,
                repo_dir,
            )?;
        }

        write_options(target, &prefs, language, options.host, locale_ids)?;

        // Finalize the git repo
        // FIXME: support tracking upstream blueprint
        //repo.remote_delete("origin")?;
        git::detached(target, repo)?;
    } else {
        return Err(Error::new(format!("Target directory is required")));
    }

    Ok(())
}
