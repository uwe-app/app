//! Helper functions to install plugins that are
//! deeply integrated with the command line tools.
use config::{
    plugin::{dependency::Dependency, Plugin, PluginSpec},
    semver::VersionReq,
};

use crate::{install_registry, new_registry, Error, Result};

static PLUGIN_DOCS: &str = "std::documentation";

async fn install_plugin(spec: PluginSpec) -> Result<Plugin> {
    let registry = new_registry()?;
    if let Some(_) = registry.spec(&spec).await? {
        let project = std::env::current_dir()?;
        let dep: Dependency = spec.into();
        Ok(install_registry(&project, &registry, &dep).await?)
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

pub async fn install_docs(range: Option<String>) -> Result<Plugin> {
    let range = if let Some(range) = range {
        range.parse::<VersionReq>()?
    } else {
        VersionReq::any()
    };
    let spec = PluginSpec::from((PLUGIN_DOCS.to_string(), range));
    Ok(install_plugin(spec).await?)
}
