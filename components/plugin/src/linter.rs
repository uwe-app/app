use regex::Regex;

use crate::{Error, Result};
use config::{features::FeatureMap, href::UrlPath, Plugin, PLUGIN_NS};

pub fn lint(plugin: &Plugin) -> Result<()> {
    let ns_re = Regex::new("^[a-zA-Z0-9_-]+$")?;

    if plugin.name.trim().is_empty() {
        return Err(Error::LintPluginNameEmpty);
    }

    if plugin.description.trim().is_empty() {
        return Err(Error::LintPluginDescriptionEmpty);
    }

    if !plugin.name.contains(PLUGIN_NS) {
        return Err(Error::LintPluginNameSpace);
    }

    for ns in plugin.name.split(PLUGIN_NS) {
        if !ns_re.is_match(ns) {
            return Err(Error::LintPluginNameInvalidNameSpace(ns.to_string()));
        }
    }

    if let Some(ref features) = plugin.features {
        lint_features(plugin, features)?;
    }

    if let Some(ref assets) = plugin.assets {
        assets.iter().try_for_each(|u| lint_path(plugin, u))?;
    }

    if let Some(ref styles) = plugin.styles {
        styles.iter().try_for_each(|s| {
            if let Some(src) = s.get_source() {
                let u = UrlPath::from(src);
                lint_path(plugin, &u)?;
            }
            Ok::<(), Error>(())
        })?;
    }

    if let Some(ref scripts) = plugin.scripts {
        scripts.iter().try_for_each(|s| {
            if let Some(src) = s.get_source() {
                let u = UrlPath::from(src);
                lint_path(plugin, &u)?;
            }
            Ok::<(), Error>(())
        })?;
    }

    if let Some(ref templates) = plugin.templates {
        for (engine, templates) in templates.iter() {
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
    }

    Ok(())
}

fn lint_path(plugin: &Plugin, path: &UrlPath) -> Result<()> {
    if path.starts_with("/") {
        return Err(Error::LintNoAbsolutePath(path.to_string()));
    }

    let buf = plugin.to_path_buf(path);
    if !buf.exists() || !buf.is_file() {
        return Err(Error::LintNoPluginFile(buf, path.to_string()));
    }
    Ok(())
}

/// Lint the feature definitions.
fn lint_features(plugin: &Plugin, map: &FeatureMap) -> Result<()> {
    let names = map.names();
    for nm in names.iter() {
        if let Some(ref deps) = plugin.dependencies {
            if deps.contains_key(nm) {
                let dep = deps.get(nm).unwrap();
                if !dep.is_optional() {
                    return Err(Error::LintFeatureDependencyNotOptional(
                        nm.to_string(),
                        dep.to_string(),
                    ));
                }
                continue;
            }
        }
        return Err(Error::LintFeatureMissing(
            plugin.to_string(),
            nm.to_string(),
        ));
    }
    Ok(())
}
