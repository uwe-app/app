use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::info;
use toml::map::Map;
use toml::value::{Table, Value};
use url::Url;

use utils::walk;

use crate::{Error, Result};
use config::{
    href::UrlPath,
    plugin::{
        dependency::{Dependency, DependencyTarget},
        Plugin, PluginSpec, PluginType,
    },
};

use plugin::{install_dependency, install_path, install_repo, new_registry};

static DEFAULT_NAME: &str = "default";
static DEFAULT_MESSAGE: &str = "Initial files.";

// Files to remove for projects created from blueprint plugins
static REMOVE: [&str; 3] = [".ignore", "plugin.orig.toml", "plugin.toml"];

#[derive(Debug)]
pub enum ProjectSource {
    Plugin(String),
    Git(Url, Option<UrlPath>),
    Path(PathBuf),
}

#[derive(Debug)]
pub struct ProjectOptions {
    pub source: Option<String>,
    pub git: Option<Url>,
    pub prefix: Option<UrlPath>,
    pub path: Option<PathBuf>,
    pub target: PathBuf,
    pub message: Option<String>,
    pub language: Option<String>,
    pub host: Option<String>,
    pub locales: Option<String>,
    pub bare: bool,
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
    plugin_name: String,
    dependency: Dependency,
    plugin: Plugin,
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

    let mut site_config: Table =
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

    // Inject a layout from a blueprint.
    if let Some(blueprint) = plugin.blueprint() {
        if let Some(ref layout) = blueprint.layout() {
            let build = site_config
                .entry("build")
                .or_insert(Value::Table(Default::default()));
            if let Value::Table(ref mut map) = build {
                map.insert(
                    "layout".to_string(),
                    Value::String(layout.to_string()),
                );
            }
        }
    }

    let empty = String::from("");

    // Inject the plugin dependency
    let dependencies = site_config
        .entry("dependencies")
        .or_insert(Value::Table(Default::default()));
    let mut dep: Map<String, Value> = Map::new();
    if let Some(ref target) = dependency.target() {
        match target {
            DependencyTarget::Repo { git, prefix } => {
                dep.insert(
                    "version".to_string(),
                    Value::String("*".to_string()),
                );
                dep.insert("git".to_string(), Value::String(git.to_string()));
                if let Some(prefix) = prefix {
                    dep.insert(
                        "prefix".to_string(),
                        Value::String(prefix.to_string()),
                    );
                }
            }
            DependencyTarget::File { path } => {
                let dest = path.canonicalize()?;
                let value = dest.to_string_lossy().into_owned().to_string();
                dep.insert(
                    "version".to_string(),
                    Value::String("*".to_string()),
                );
                dep.insert("path".to_string(), Value::String(value));
            }
            _ => {}
        }
    } else {
        dep.insert(
            "version".to_string(),
            Value::String(plugin.version().to_string()),
        );
    }

    if let Value::Table(ref mut map) = dependencies {
        map.insert(plugin_name, Value::Table(dep));
    }

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

pub async fn project(mut options: ProjectOptions) -> Result<()> {
    let mut sources: Vec<ProjectSource> = Vec::new();
    if let Some(git) = options.git.take() {
        sources.push(ProjectSource::Git(git, options.prefix.clone()))
    }
    if let Some(path) = options.path.take() {
        sources.push(ProjectSource::Path(path))
    }
    if let Some(plugin) = options.source.take() {
        sources.push(ProjectSource::Plugin(plugin))
    }

    if sources.is_empty() {
        sources.push(ProjectSource::Plugin(DEFAULT_NAME.to_string()));
    }

    if sources.len() > 1 {
        return Err(Error::NewProjectMultipleSource);
    }

    let source = sources.swap_remove(0);

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

    let (name, dependency) = match source {
        ProjectSource::Git(git, prefix) => (
            None,
            Dependency::new_target(DependencyTarget::Repo { git, prefix }),
        ),
        ProjectSource::Path(path) => (
            None,
            Dependency::new_target(DependencyTarget::File { path }),
        ),
        ProjectSource::Plugin(plugin_name) => {
            let spec: PluginSpec =
                if let Ok(spec) = plugin_name.parse::<PluginSpec>() {
                    spec
                } else {
                    let fqn = format!(
                        "{}{}{}",
                        config::PLUGIN_BLUEPRINT_NAMESPACE,
                        config::PLUGIN_NS,
                        plugin_name
                    );
                    PluginSpec::from(fqn)
                };

            (
                Some(spec.name().to_string()),
                Dependency::new(spec.range().clone()),
            )
        }
    };

    let registry = new_registry()?;
    let (name, mut plugin) = if let Some(ref name) = name {
        (
            name.to_string(),
            install_dependency(
                &target,
                &registry,
                name,
                &dependency,
                false,
                None,
            )
            .await?,
        )
    // Now we have a dependency but no plugin name yet!
    } else {
        let dep_target: DependencyTarget = dependency.target().clone().unwrap();
        match dep_target {
            DependencyTarget::Repo {
                ref git,
                ref prefix,
            } => {
                let plugin = install_repo(&target, git, prefix, true).await?;
                (plugin.name().to_string(), plugin)
            }
            DependencyTarget::File { ref path } => {
                let plugin = install_path(&target, path, None).await?;
                (plugin.name().to_string(), plugin)
            }
            _ => {
                panic!("Unsupported dependency target whilst creating new project!")
            }
        }
    };

    let source_dir = plugin.base().to_path_buf();
    if !source_dir.exists() || !source_dir.is_dir() {
        return Err(Error::NotDirectory(source_dir.to_path_buf()));
    }

    if plugin.kind() != &PluginType::Blueprint {
        return Err(Error::BlueprintPluginInvalidType(
            plugin.name().to_string(),
            plugin.version().to_string(),
            plugin.kind().to_string(),
        ));
    }

    check_site_settings(&source_dir)?;
    create_target_parents(&target)?;

    // Compile the matching directives
    if let Some(directives) = plugin.blueprint_mut() {
        if let Some(files) = directives.files_mut() {
            files.compile();
        }
    }

    walk::copy(&source_dir, &target, |f| {
        // Built in files we always want to ignore.
        if let Some(file_name) = f.file_name() {
            let name = file_name.to_string_lossy();
            if REMOVE.contains(&name.as_ref()) {
                return false;
            }
        }

        // Filter files based on the blueprint directives
        // in the plugin.
        if let Some(directives) = plugin.blueprint() {
            if let Some(files) = directives.files() {
                if let Ok(relative) = f.strip_prefix(&source_dir) {
                    let rel_path: UrlPath = UrlPath::from(relative);
                    if files.is_excluded(rel_path.as_str())
                        && !files.is_included(rel_path.as_str())
                    {
                        return false;
                    }
                }
            }
        }

        true
    })?;

    write_settings(&target, settings, name, dependency, plugin)?;

    if !options.bare {
        scm::init(&target, message)?;
        if let Some(ref url) = options.remote_url {
            scm::set_remote(&target, &options.remote_name, url)?;
        }
    }

    info!("Created {} ✓", target.to_string_lossy());

    Ok(())
}
