use std::path::PathBuf;

use collator::{create_file, CollateInfo};
use config::{
    DependencyMap,
    Plugin,
    RuntimeOptions,
    ASSETS,
    PLUGINS,
};

use crate::{Error, Result};

/// Inject synthetic files for plugin assets.
fn assets(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    name: &String,
    plugin: &Plugin,
    plugin_target: &PathBuf) -> Result<()> {

    if let Some(ref assets) = plugin.assets {
        for asset in assets {
            if asset.is_absolute() {
                return Err(
                    Error::PluginAbsolutePath(
                        name.clone(), asset.to_path_buf()));
            }

            let asset_source = plugin.base.join(asset);
            if !asset_source.exists() {
                return Err(Error::NoPluginAsset(name.clone(), asset_source));
            }

            let asset_rel = asset_source.strip_prefix(&plugin.base)?.to_path_buf();
            let asset_target = plugin_target.join(&asset_rel);
            let asset_href = utils::url::to_href_separator(&asset_target);
            let asset_href = format!("/{}", asset_href.trim_start_matches("/"));

            create_file(
                options,
                info,
                asset_source,
                asset_target,
                asset_href,
                None,
            )?;
        }
    }

    Ok(())
}

/// Inject synthetic files for script assets.
fn scripts(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    name: &String,
    plugin: &Plugin,
    plugin_target: &PathBuf) -> Result<()> {

    if let Some(ref scripts) = plugin.scripts {
        println!("Scripts {:#?}", scripts);

        for script in scripts {

            //if asset.is_absolute() {
                //return Err(
                    //Error::PluginAbsolutePath(
                        //name.clone(), asset.to_path_buf()));
            //}

            //let asset_source = plugin.base.join(asset);
            //if !asset_source.exists() {
                //return Err(Error::NoPluginAsset(name.clone(), asset_source));
            //}

            //let asset_rel = asset_source.strip_prefix(&plugin.base)?.to_path_buf();
            //let asset_target = plugin_target.join(&asset_rel);
            //let asset_href = utils::url::to_href_separator(&asset_target);
            //let asset_href = format!("/{}", asset_href.trim_start_matches("/"));

            //create_file(
                //options,
                //info,
                //asset_source,
                //asset_target,
                //asset_href,
                //None,
            //)?;
        }
    }

    Ok(())
}

pub fn collate(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    plugins: &DependencyMap) -> Result<()> {

    let assets_base = PathBuf::from(ASSETS).join(PLUGINS);

    for (name, dep) in plugins.to_vec() {
        let plugin = dep.plugin.as_ref().unwrap();
        let plugin_base = assets_base.join(name);

        assets(options, info, name, plugin, &plugin_base)?;
        scripts(options, info, name, plugin, &plugin_base)?;
    }

    std::process::exit(1);

    Ok(())
}
