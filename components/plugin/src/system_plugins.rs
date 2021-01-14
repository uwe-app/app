//! Helper functions to install plugins that are
//! deeply integrated with the command line tools.
//!
use semver::{Version, VersionReq};

use config::{
    plugin::{dependency::Dependency, Plugin, PluginSpec},
};

use crate::{installer::install_registry, new_registry, Error, Result};

static PLUGIN_DOCS: &str = "std::documentation";

async fn install_plugin(spec: PluginSpec) -> Result<Plugin> {
    let registry = new_registry()?;
    if let Some(_) = registry.spec(&spec).await? {
        let project = std::env::current_dir()?;
        let name = spec.name().to_string();
        let dep: Dependency = spec.into();
        Ok(install_registry(&project, &registry, &name, &dep).await?)
    } else {
        Err(Error::PluginNotFound(spec.to_string()))
    }
}

pub async fn install_blueprint(source: &str) -> Result<Plugin> {
    let spec: PluginSpec = if let Ok(spec) = source.parse::<PluginSpec>() {
        spec
    } else {
        let fqn = format!(
            "{}{}{}",
            config::PLUGIN_BLUEPRINT_NAMESPACE,
            config::PLUGIN_NS,
            source
        );
        PluginSpec::from(fqn)
    };
    Ok(install_plugin(spec).await?)
}

/// Install the offline documentation plugin attempting to use
/// the preferred version if it is available in the registry 
/// otherwise fallback to the latest available version.
pub async fn install_docs(prefers_version: Option<&Version>) -> Result<Plugin> {
    let registry = new_registry()?;
    let entry = registry.entry(PLUGIN_DOCS).await?;

    let latest = PluginSpec::from((PLUGIN_DOCS.to_string(), VersionReq::any()));
    let spec = if let Some(entry) = entry {
        if let Some(version) = prefers_version {
            if let Some(_) = entry.get(version) {
                PluginSpec::from((PLUGIN_DOCS.to_string(), VersionReq::exact(version)))    
            } else { latest }
        } else {
            latest 
        }
    } else { latest };

    Ok(install_plugin(spec).await?)
}
