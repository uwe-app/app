use async_recursion::async_recursion;

use crate::{Error, Result};
use config::{Dependency, DependencyMap, Plugin};

static PLUGIN: &str = "plugin.toml";

async fn load(dep: &Dependency) -> Result<Plugin> {
    let path = if let Some(ref path) = dep.path {
        path.to_path_buf()
    } else {
        todo!();
    };

    let plugin_file = path.join(PLUGIN);
    let plugin_content = utils::fs::read_string(plugin_file)?;
    let mut plugin: Plugin = toml::from_str(&plugin_content)?;
    plugin.base = path;
    Ok(plugin)
}

#[async_recursion]
pub async fn solve(
    input: DependencyMap,
    output: &mut DependencyMap,
    stack: &mut Vec<String>,
) -> Result<()> {
    for (name, mut dep) in input.into_iter() {
        let mut plugin = load(&dep).await?;

        if stack.contains(&plugin.name) {
            return Err(Error::PluginCyclicDependency(plugin.name.clone()));
        }

        if !dep.version.matches(&plugin.version) {
            return Err(Error::PluginVersionMismatch(
                plugin.name.clone(),
                plugin.version.to_string(),
                dep.version.to_string(),
            ));
        }

        stack.push(plugin.name.clone());

        if let Some(dependencies) = plugin.dependencies.take() {
            let mut deps: DependencyMap = Default::default();
            solve(dependencies, &mut deps, stack).await?;
            //dep.plugin.as_mut().dependencies = Some(deps);
        }

        dep.plugin = Some(plugin);

        output.items.insert(name, dep);
    }

    Ok(())
}
