use std::collections::HashMap;

use globset::GlobMatcher;

use crate::{
    dependency::Dependency, engine::TemplateEngine, plugin::ResolvedPlugins,
    script::ScriptAsset, style::StyleAsset, Error, Result,
};

#[derive(Debug, Clone, Default)]
pub struct PluginCache {
    // Computed plugins
    plugins: ResolvedPlugins,

    // Cache of plugin dependencies that should be applied to pages
    styles_cache: Vec<(Dependency, Vec<StyleAsset>)>,
    scripts_cache: Vec<(Dependency, Vec<ScriptAsset>)>,
    layouts_cache: HashMap<String, Vec<GlobMatcher>>,
}

impl PluginCache {
    pub fn new(plugins: ResolvedPlugins) -> Self {
        Self {
            plugins,
            styles_cache: Vec::new(),
            scripts_cache: Vec::new(),
            layouts_cache: HashMap::new(),
        }
    }

    pub fn plugins(&self) -> &ResolvedPlugins {
        &self.plugins
    }

    pub fn plugins_mut(&mut self) -> &mut ResolvedPlugins {
        &mut self.plugins
    }

    pub fn styles(&self) -> &Vec<(Dependency, Vec<StyleAsset>)> {
        &self.styles_cache
    }

    pub fn scripts(&self) -> &Vec<(Dependency, Vec<ScriptAsset>)> {
        &self.scripts_cache
    }

    pub fn layouts(&self) -> &HashMap<String, Vec<GlobMatcher>> {
        &self.layouts_cache
    }

    // FIXME: stricter error handling on mismatch
    pub fn prepare(&mut self, engine: &TemplateEngine) -> Result<()> {
        for (dep, plugin) in self.plugins.iter_mut() {
            if let Some(ref apply) = dep.apply {
                let assets_href_base = format!(
                    "/{}",
                    utils::url::to_href_separator(plugin.to_assets_path())
                );

                if !plugin.styles().is_empty() && !apply.styles_match.is_empty()
                {
                    // Make style paths relative to the plugin asset destination
                    let styles = plugin
                        .styles()
                        .clone()
                        .into_iter()
                        .map(|mut s| {
                            s.set_source_prefix(&assets_href_base);
                            s
                        })
                        .collect::<Vec<StyleAsset>>();
                    self.styles_cache.push((dep.clone(), styles));
                }
                if !plugin.scripts().is_empty()
                    && !apply.scripts_match.is_empty()
                {
                    let scripts = plugin
                        .scripts()
                        .clone()
                        .into_iter()
                        .map(|mut s| {
                            s.set_source_prefix(&assets_href_base);
                            s
                        })
                        .collect::<Vec<ScriptAsset>>();
                    self.scripts_cache.push((dep.clone(), scripts));
                }

                // Got some layouts to apply so add to the cache
                if !apply.layouts_match.is_empty() {
                    let templates =
                        plugin.templates().get(engine).ok_or_else(|| {
                            Error::ApplyLayoutNoTemplateForEngine(
                                dep.to_string(),
                                engine.to_string(),
                            )
                        })?;
                    let layouts =
                        templates.layouts.as_ref().ok_or_else(|| {
                            Error::ApplyLayoutNoLayouts(
                                dep.to_string(),
                                engine.to_string(),
                            )
                        })?;

                    for (key, matches) in apply.layouts_match.iter() {
                        if !layouts.contains_key(key) {
                            return Err(Error::ApplyLayoutNoLayoutForKey(
                                dep.to_string(),
                                engine.to_string(),
                                key.clone(),
                            ));
                        }
                        let fqn = plugin.qualified(key);
                        self.layouts_cache.insert(fqn, matches.clone());
                    }
                }
            }
        }

        Ok(())
    }
}
