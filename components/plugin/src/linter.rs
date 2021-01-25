use std::fs;
use std::path::Path;

use regex::Regex;
use spdx::license_id;

use bracket::{parser::ParserOptions, template::Template};

use config::{
    features::FeatureMap,
    href::UrlPath,
    license::{License, LicenseGroup},
    Config, Plugin, PluginType, PLUGIN_NS,
};

use utils::walk;

use crate::{compute, error::LintError, reader::read};

pub async fn lint<P: AsRef<Path>>(path: P) -> crate::Result<Plugin> {
    let plugin = read(path).await?;
    let plugin = compute::transform(&plugin).await?;
    lint_plugin(&plugin)?;
    Ok(plugin)
}

pub(crate) fn lint_plugin(plugin: &Plugin) -> crate::Result<()> {
    let result = match plugin.kind() {
        PluginType::Library => lint_library(plugin),
        PluginType::Blueprint => lint_blueprint(plugin),
    };
    Ok(result.map_err(crate::Error::from)?)
}

/// Lint common to all plugin types.
fn lint_common(plugin: &Plugin) -> Result<(), LintError> {
    let ns_re = Regex::new("^[a-zA-Z0-9-]+$")?;

    if plugin.name().trim().is_empty() {
        return Err(LintError::LintPluginNameEmpty);
    }

    if plugin.description().trim().is_empty() {
        return Err(LintError::LintPluginDescriptionEmpty);
    }

    if !plugin.name().contains(PLUGIN_NS) {
        return Err(LintError::LintPluginNameSpace);
    }

    for ns in plugin.name().split(PLUGIN_NS) {
        if !ns_re.is_match(ns) {
            return Err(LintError::LintPluginNameInvalidNameSpace(
                ns.to_string(),
            ));
        }
    }

    lint_licenses(plugin)?;

    lint_symlinks(plugin)?;

    Ok(())
}

/// Walk all files and check for symbolic links.
fn lint_symlinks(plugin: &Plugin) -> Result<(), LintError> {
    let base = plugin.base().canonicalize()?;
    let git = base.join(".git");
    let files = walk::find(plugin.base(), |f| {
        if let Ok(file) = f.canonicalize() {
            if file.starts_with(&git) {
                return false;
            }
        }
        true
    });
    for f in files {
        if let Ok(abs) = f.canonicalize() {
            if !abs.starts_with(&base) {
                return Err(LintError::LintSymbolicLink(
                    abs,
                    base.to_path_buf(),
                ));
            }
        }
    }
    Ok(())
}

/// Lint for the blueprint plugin type.
fn lint_blueprint(plugin: &Plugin) -> Result<(), LintError> {
    lint_common(plugin)?;
    if !plugin.features().is_empty() {
        return Err(LintError::LintFeaturesSiteType);
    }
    let _ = Config::load_config(plugin.base())?;
    Ok(())
}

/// Lint for the library plugin type.
fn lint_library(plugin: &Plugin) -> Result<(), LintError> {
    lint_common(plugin)?;

    if plugin.blueprint().is_some() {
        return Err(LintError::LintBlueprintNotAllowed);
    }

    if !plugin.features().is_empty() {
        lint_features(plugin, plugin.features())?;
    }

    plugin
        .assets()
        .iter()
        .try_for_each(|u| lint_path(plugin, u))?;

    plugin.styles().iter().try_for_each(|s| {
        if let Some(src) = s.source() {
            let u = UrlPath::from(src);
            lint_path(plugin, &u)?;
        }
        Ok::<(), LintError>(())
    })?;

    plugin.scripts().iter().try_for_each(|s| {
        if let Some(src) = s.source() {
            let u = UrlPath::from(src);
            lint_path(plugin, &u)?;
        }
        Ok::<(), LintError>(())
    })?;

    //if let Some(ref templates) = plugin.templates {
    for (_engine, templates) in plugin.templates().iter() {
        if let Some(ref partials) = templates.partials {
            for (_, asset) in partials {
                lint_path(plugin, &asset.file)?;
                lint_template(plugin, &asset.file)?;
            }
        }
        if let Some(ref layouts) = templates.layouts {
            for (_, asset) in layouts {
                lint_path(plugin, &asset.file)?;
                lint_template(plugin, &asset.file)?;
            }
        }
    }
    //}

    Ok(())
}

fn lint_licenses(plugin: &Plugin) -> Result<(), LintError> {
    if let Some(ref license) = plugin.license() {
        lint_license(license)?;
    }

    for (_, v) in plugin.library() {
        if let Some(ref license) = v.license() {
            lint_license(license)?;
        }
    }

    Ok(())
}

fn lint_license(license: &LicenseGroup) -> Result<(), LintError> {
    for license in license.to_vec() {
        match license {
            License::Spdx(ref value) => {
                if let None = license_id(value) {
                    return Err(LintError::LintLicenseNotSpdx(
                        value.to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn lint_path(plugin: &Plugin, path: &UrlPath) -> Result<(), LintError> {
    if path.as_str().starts_with("/") {
        return Err(LintError::LintNoAbsolutePath(path.to_string()));
    }

    let buf = plugin.to_path_buf(path);
    if !buf.exists() || !buf.is_file() {
        return Err(LintError::LintNoPluginFile(buf, path.to_string()));
    }
    Ok(())
}

/// Lint the feature definitions.
fn lint_features(plugin: &Plugin, map: &FeatureMap) -> Result<(), LintError> {
    let names = map.names();
    'names: for nm in names {
        if let Some(ref dep) = plugin.dependencies().get(nm) {
            if !dep.is_optional() {
                return Err(LintError::LintFeatureDependencyNotOptional(
                    nm.to_string(),
                    dep.to_string(),
                ));
            }
            continue;
        }

        for (_, p) in plugin.plugins() {
            if &p.name == nm {
                continue 'names;
            }
        }

        return Err(LintError::LintFeatureMissing(
            plugin.to_string(),
            nm.to_string(),
        ));
    }
    Ok(())
}

fn lint_template(plugin: &Plugin, path: &UrlPath) -> Result<(), LintError> {
    let buf = plugin.to_path_buf(path);
    let content = fs::read_to_string(&buf)?;
    let file_name = buf.to_string_lossy().into_owned().to_string();
    let options = ParserOptions::new(file_name, 0, 0);
    let _ = Template::compile(content, options)?;
    Ok(())
}
