use std::path::Path;
use regex::Regex;
use spdx::license_id;

use crate::{error::LintError, reader::read, compute};
use config::{
    features::FeatureMap,
    href::UrlPath,
    Plugin, PLUGIN_NS,
    license::{License, LicenseGroup},
};

pub async fn lint<P: AsRef<Path>>(path: P) -> crate::Result<Plugin> {
    let plugin = read(path).await?;
    let plugin = compute::transform(&plugin).await?;
    lint_plugin(&plugin)?;
    Ok(plugin)
}

pub(crate) fn lint_plugin(plugin: &Plugin) -> crate::Result<()> {
    Ok(run(&plugin).map_err(crate::Error::from)?)
}

fn run(plugin: &Plugin) -> Result<(), LintError> {
    let ns_re = Regex::new("^[a-zA-Z0-9-]+$")?;

    if plugin.name.trim().is_empty() {
        return Err(LintError::LintPluginNameEmpty);
    }

    if plugin.description.trim().is_empty() {
        return Err(LintError::LintPluginDescriptionEmpty);
    }

    if !plugin.name.contains(PLUGIN_NS) {
        return Err(LintError::LintPluginNameSpace);
    }

    for ns in plugin.name.split(PLUGIN_NS) {
        if !ns_re.is_match(ns) {
            return Err(LintError::LintPluginNameInvalidNameSpace(ns.to_string()));
        }
    }

    lint_licenses(plugin)?;

    if let Some(ref features) = plugin.features {
        lint_features(plugin, features)?;
    }

    plugin.assets().iter().try_for_each(|u| lint_path(plugin, u))?;

    plugin.styles().iter().try_for_each(|s| {
        if let Some(src) = s.get_source() {
            let u = UrlPath::from(src);
            lint_path(plugin, &u)?;
        }
        Ok::<(), LintError>(())
    })?;

    if let Some(ref scripts) = plugin.scripts {
        scripts.iter().try_for_each(|s| {
            if let Some(src) = s.get_source() {
                let u = UrlPath::from(src);
                lint_path(plugin, &u)?;
            }
            Ok::<(), LintError>(())
        })?;
    }

    //if let Some(ref templates) = plugin.templates {
    for (_engine, templates) in plugin.templates.iter() {
        if let Some(ref partials) = templates.partials {
            for (_, asset) in partials {
                lint_path(plugin, &asset.file)?;
            }
        }
        if let Some(ref layouts) = templates.layouts {
            for (_, asset) in layouts {
                lint_path(plugin, &asset.file)?;
            }
        }
    }
    //}

    Ok(())
}

fn lint_licenses(plugin: &Plugin) -> Result<(), LintError> {
    if let Some(ref license) = plugin.license {
        lint_license(license)?;
    }

    if let Some(ref library) = plugin.library {
        for (_, v) in library {
            if let Some(ref license) = v.license {
                lint_license(license)?;
            }
        }
    }

    Ok(())
}

fn lint_license(license: &LicenseGroup) -> Result<(), LintError> {
    for license in license.to_vec() {
        match license {
            License::Spdx(ref value) => {
                if let None = license_id(value) {
                    return Err(
                        LintError::LintLicenseNotSpdx(
                            value.to_string()));
                }
            }
        }
    }

    Ok(())
}

fn lint_path(plugin: &Plugin, path: &UrlPath) -> Result<(), LintError> {
    if path.starts_with("/") {
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
    for nm in names.iter() {
        if let Some(ref deps) = plugin.dependencies {
            if deps.contains_key(nm) {
                let dep = deps.get(nm).unwrap();
                if !dep.is_optional() {
                    return Err(LintError::LintFeatureDependencyNotOptional(
                        nm.to_string(),
                        dep.to_string(),
                    ));
                }
                continue;
            }
        }
        return Err(LintError::LintFeatureMissing(
            plugin.to_string(),
            nm.to_string(),
        ));
    }
    Ok(())
}
