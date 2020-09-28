use regex::Regex;

use crate::{Error, Result};
use config::{Plugin, PLUGIN_NS, features::FeatureMap};

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

    Ok(())
}

/// Lint the feature definitions.
fn lint_features(plugin: &Plugin, map: &FeatureMap) -> Result<()> {
    let names = Plugin::features(map);
    for nm in names.iter() {
        if let Some(ref deps) = plugin.dependencies {
            if deps.contains_key(nm) {
                let dep = deps.get(nm).unwrap();
                if !dep.is_optional() {
                    return Err(
                        Error::LintFeatureDependencyNotOptional(
                            nm.to_string(), dep.to_string()));
                }
                continue;
            }
        }
        return Err(
            Error::LintFeatureMissing(plugin.to_string(), nm.to_string()));
    }
    Ok(())
}
