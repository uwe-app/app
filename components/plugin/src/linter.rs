use regex::Regex;

use config::{Plugin, PLUGIN_NS};
use crate::{Error, Result};

pub fn lint(plugin: &Plugin) -> Result<()> {
    let ns_re = Regex::new("^[a-zA-Z0-9_-]+$")?;

    if plugin.name.trim().is_empty() {
        return Err(Error::LintPluginNameEmpty)
    }

    if plugin.description.trim().is_empty() {
        return Err(Error::LintPluginDescriptionEmpty)
    }

    if !plugin.name.contains(PLUGIN_NS) {
        return Err(Error::LintPluginNameSpace)
    }

    for ns in plugin.name.split(PLUGIN_NS) {
        if !ns_re.is_match(ns) {
            return Err(
                Error::LintPluginNameInvalidNameSpace(ns.to_string()))
        }
    }

    Ok(())
}
