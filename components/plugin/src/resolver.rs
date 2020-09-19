use async_recursion::async_recursion;

use config::{semver::Version, Dependency, DependencyMap, Plugin};
use crate::{Error, Result};

static PLUGIN: &str = "plugin.toml";

async fn load(dep: &Dependency) -> Result<Plugin> {
    let path = if let Some(ref path) = dep.path {
        path.to_path_buf()
    } else {
        todo!();
    };

    let plugin_file = path.join(PLUGIN);
    let plugin_content = utils::fs::read_string(plugin_file)?;
    Ok(toml::from_str(&plugin_content)?)
}

#[async_recursion]
pub async fn solve(input: &mut DependencyMap, stack: &mut Vec<String>) -> Result<()> {

    for(name, dep) in input.iter_mut() {
        let mut plugin = load(&dep).await?;

        if stack.contains(&plugin.name) {
            return Err(Error::PluginCyclicDependency(plugin.name.clone()));
        }

        if !dep.version.matches(&plugin.version) {
            return Err(Error::PluginVersionMismatch(
                plugin.name.clone(),
                plugin.version.to_string(),
                dep.version.to_string()));
        }

        stack.push(plugin.name.clone());

        println!("Got plugin {:?}", plugin);

        if let Some(ref mut dependencies) = plugin.dependencies {
            println!("Got nested dependencies");
            solve(dependencies, stack).await?;
        }

        dep.plugin = Some(plugin)


        //let resolved = ResolvedPlugin::new(dep, plugin);
        //output.entry(name.clone()).or_insert(resolved);
    }

    std::process::exit(1);

    Ok(())
}
