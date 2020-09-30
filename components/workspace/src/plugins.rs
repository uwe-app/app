use std::path::PathBuf;
use std::sync::Arc;

use collator::{create_file, CollateInfo};
use config::{plugin::ResolvedPlugins, Config, Plugin, RuntimeOptions};

use crate::{Error, Result};

/// Helper to create a synthetic asset from the plugin.
fn create_asset<S: AsRef<str>>(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    name: &String,
    plugin: &Plugin,
    plugin_target: &PathBuf,
    asset: S,
) -> Result<()> {
    let asset = asset.as_ref();

    let asset = PathBuf::from(utils::url::to_path_separator(
        asset.trim_start_matches("/"),
    ));

    if asset.is_absolute() {
        return Err(Error::PluginAbsolutePath(
            name.clone(),
            asset.to_path_buf(),
        ));
    }

    let asset_source = plugin.base().join(&asset);
    if !asset_source.exists() {
        return Err(Error::NoPluginAsset(name.clone(), asset_source));
    }

    let asset_rel = asset_source.strip_prefix(plugin.base())?.to_path_buf();
    let asset_target = plugin_target.join(&asset_rel);
    let asset_href = utils::url::to_href_separator(&asset_target);
    let asset_href = format!("/{}", asset_href.trim_start_matches("/"));

    create_file(options, info, asset_source, asset_target, asset_href, None)?;

    Ok(())
}

/// Inject file assets.
fn assets(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    name: &String,
    plugin: &Plugin,
    plugin_target: &PathBuf,
) -> Result<()> {
    if let Some(ref assets) = plugin.assets {
        for asset in assets {
            create_asset(options, info, name, plugin, plugin_target, asset)?;
        }
    }
    Ok(())
}

/// Inject script assets.
fn scripts(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    name: &String,
    plugin: &Plugin,
    plugin_target: &PathBuf,
) -> Result<()> {
    if let Some(ref scripts) = plugin.scripts {
        for script in scripts {
            if let Some(src) = script.get_source() {
                create_asset(options, info, name, plugin, plugin_target, src)?;
            }
        }
    }
    Ok(())
}

/// Inject style assets.
fn styles(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    name: &String,
    plugin: &Plugin,
    plugin_target: &PathBuf,
) -> Result<()> {
    if let Some(ref styles) = plugin.styles {
        for style in styles {
            if let Some(src) = style.get_source() {
                create_asset(options, info, name, plugin, plugin_target, src)?;
            }
        }
    }
    Ok(())
}

/// Inject template layouts.
fn layouts(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    name: &String,
    plugin: &Plugin,
) -> Result<()> {
    if let Some(ref templates) = plugin.templates.get(config.engine()) {
        if let Some(ref layouts) = templates.layouts {
            for (nm, layout) in layouts.iter() {
                let fqn = plugin.qualified(nm);
                let layout_path = layout.to_path_buf(plugin.base());
                info.add_layout(fqn, Arc::new(layout_path));
            }
        }
    }
    Ok(())
}

/// Add plugin files to the collation.
pub fn collate(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    plugins: &ResolvedPlugins,
) -> Result<()> {
    for (_dep, plugin) in plugins.iter() {
        let name = &plugin.name;
        let plugin_base = plugin.assets();

        assets(options, info, name, plugin, &plugin_base)?;
        scripts(options, info, name, plugin, &plugin_base)?;
        styles(options, info, name, plugin, &plugin_base)?;
        layouts(config, options, info, name, plugin)?;
    }

    Ok(())
}
