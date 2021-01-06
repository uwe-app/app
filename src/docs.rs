use std::path::PathBuf;

use semver::VersionReq;

use crate::{Error, Result};
use config::{
    plugin::{PluginSpec, dependency::Dependency},
    server::{LaunchConfig, ServerConfig}
};
use plugin::{new_registry, install_registry};

static PUBLIC_HTML: &str = "public_html";
static PLUGIN_NAMESPACE: &str = "std::documentation";

pub async fn target(range: Option<String>) -> Result<PathBuf> {
    let registry = new_registry()?;
    let range = if let Some(range) = range {
        range.parse::<VersionReq>()?
    } else { VersionReq::any() };

    let spec = PluginSpec::from((PLUGIN_NAMESPACE.to_string(), range));
    if let Some(_) = registry.spec(&spec).await? {
        let project = std::env::current_dir()?;
        let dep: Dependency = spec.into();
        let plugin = install_registry(&project, &registry, &dep).await?;
        Ok(plugin.base().join(PUBLIC_HTML))
    } else {
        Err(Error::DocumentationPluginNotFound(spec.to_string()))
    }
}

pub async fn open(opts: ServerConfig) -> Result<()> {
    let launch = LaunchConfig { open: true };

    // Convert to &'static reference
    let opts = server::configure(opts);
    Ok(server::launch(opts, launch).await?)
}
