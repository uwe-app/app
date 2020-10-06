use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::fs;

use slug::slugify;

use config::{
    engine::{TemplateEngine, ENGINES},
    href::UrlPath,
    plugin::TemplateAsset,
    script::ScriptAsset,
    style::StyleAsset,
    plugin::Plugin,
    dependency::Dependency,
    semver::VersionReq,
};

use utils::walk;

use crate::{Error, Result};

static PLUGIN_STACK_SIZE: usize = 8;

/// Compute plugin information by convention from the file system.
pub(crate) async fn transform(original: &Plugin) -> Result<Plugin> {
    let mut computed = original.clone();
    let base = computed.base().canonicalize()?;

    let mut stack: Vec<PathBuf> = Vec::new();

    let prefix = PathBuf::new();
    load_scope(base, prefix, &mut computed, &mut stack)?;

    //println!("{:#?}", computed);

    Ok(computed)
}

fn load_scope(base: PathBuf, prefix: PathBuf, scope: &mut Plugin, stack: &mut Vec<PathBuf>) -> Result<()> {

    if stack.len() > PLUGIN_STACK_SIZE {
        return Err(Error::PluginStackTooLarge(PLUGIN_STACK_SIZE));
    }

    let assets = base.join(config::ASSETS);
    let fonts = base.join(config::FONTS);
    let styles = base.join(config::STYLES);
    let scripts = base.join(config::SCRIPTS);
    let plugins = base.join(config::PLUGINS);

    if assets.exists() && assets.is_dir() {
        load_assets(&base, &prefix, &assets, scope);
    }

    // Fonts just get placed in the assets collection, this
    // convention is for convenience so plugin authors can
    // be more explicit using the file system layout.
    if fonts.exists() && fonts.is_dir() {
        load_assets(&base, &prefix, &fonts, scope);
    }

    if styles.exists() && styles.is_dir() {
        load_styles(&base, &prefix, &styles, scope);
    }

    if scripts.exists() && scripts.is_dir() {
        load_scripts(&base, &prefix, &scripts, scope);
    }

    for engine in ENGINES.iter() {
        load_engine(&base, &prefix, &engine, scope);
    }

    if plugins.exists() && plugins.is_dir() {
        for entry in fs::read_dir(plugins)? {
            let path = entry?.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    let dir_name = name.to_string_lossy().into_owned();
                    let scope_name = slugify(dir_name);

                    let scope_base = path.to_path_buf().canonicalize()?;

                    if stack.contains(&scope_base) {
                        return Err(Error::CyclicPlugin(base));
                    }

                    let scope_prefix = scope_base.strip_prefix(&base)?.to_path_buf();
                    let prefix = UrlPath::from(&scope_prefix);
                    let mut child_scope: Plugin = Plugin::new_scope(scope, &scope_name, prefix);

                    stack.push(scope_base.clone());
                    load_scope(scope_base, scope_prefix, &mut child_scope, stack)?;

                    let feature_name = scope_name.clone();
                    let dependency_name = child_scope.name.clone();

                    // NOTE: If a plugin with the same name has already been
                    // NOTE: defined manually then it will take precedence.
                    scope.plugins_mut().entry(scope_name.clone()).or_insert(child_scope);

                    // Create a feature for the scoped plugin
                    let features = scope.features_mut();
                    let features_list = features.entry(feature_name).or_insert(Vec::new());
                    features_list.push(dependency_name.clone());

                    // Create an optional local scoped dependency for the
                    // plugin so it can be resolved at build time
                    let version_req = VersionReq::exact(&scope.version);
                    let dependency = Dependency::new_scope(
                        scope_name.clone(),
                        version_req);
                    scope.dependencies_mut()
                        .entry(dependency_name).or_insert(dependency);

                    stack.pop();
                }
            }
        }
    }

    // TODO: support computing for `pages` and `files`

    Ok(())
}

fn load_assets(base: &PathBuf, prefix: &PathBuf, dir: &Path, computed: &mut Plugin) {
    let files = walk::find(dir, |_| true);
    if !files.is_empty() {
        let items = files
            .iter()
            .filter(|e| e.is_file())
            .map(|e| UrlPath::from(prefix.join(e.strip_prefix(&base).unwrap())))
            .collect::<HashSet<_>>();

        let existing = computed.assets();
        let assets: HashSet<_> = items.union(&existing).cloned().collect();
        computed.set_assets(assets);
    }
}

fn load_styles(base: &PathBuf, prefix: &PathBuf, dir: &Path, computed: &mut Plugin) {
    let ext = OsStr::new("css");
    let files = walk::find(dir, |e| {
        if let Some(extension) = e.extension() {
            return extension == ext;
        }
        false
    });
    if !files.is_empty() {
        let mut items = files
            .iter()
            .filter(|e| e.is_file())
            .map(|e| {
                StyleAsset::from(UrlPath::from(prefix.join(e.strip_prefix(&base).unwrap())))
            })
            .collect::<Vec<_>>();

        let mut existing = computed.styles_mut();

        items.append(&mut existing);

        // NOTE: Normalize to tags so that we avoid the TOML
        // NOTE: error 'values must be emitted before tables'
        items = items
            .iter()
            .map(|s| StyleAsset::Tag(s.to_tag()))
            .collect::<Vec<_>>();

        // Ensure we don't have any duplicates
        let mut uniques = HashSet::new();
        items.retain(|e| uniques.insert(e.clone()));

        computed.set_styles(items);
    }
}

fn load_scripts(base: &PathBuf, prefix: &PathBuf, dir: &Path, computed: &mut Plugin) {
    let ext = OsStr::new("js");
    let files = walk::find(dir, |e| {
        if let Some(extension) = e.extension() {
            return extension == ext;
        }
        false
    });
    if !files.is_empty() {
        let mut items = files
            .iter()
            .filter(|e| e.is_file())
            .map(|e| {
                ScriptAsset::from(UrlPath::from(prefix.join(e.strip_prefix(&base).unwrap())))
            })
            .collect::<Vec<_>>();

        let mut existing = computed.scripts_mut();
        items.append(&mut existing);

        // NOTE: Normalize to tags so that we avoid the TOML
        // NOTE: error 'values must be emitted before tables'
        items = items
            .iter()
            .map(|s| ScriptAsset::Tag(s.to_tag()))
            .collect::<Vec<_>>();

        // Ensure we don't have any duplicates
        let mut uniques = HashSet::new();
        items.retain(|e| uniques.insert(e.clone()));

        computed.set_scripts(items);
    }
}

fn load_engine(
    base: &PathBuf,
    prefix: &PathBuf,
    engine: &TemplateEngine,
    computed: &mut Plugin,
) {
    let partials = base.join(config::PARTIALS);
    let layouts = base.join(config::LAYOUTS);
    if partials.exists() && partials.is_dir() {
        load_partials(base, prefix, &partials, engine, computed);
    }
    if layouts.exists() && layouts.is_dir() {
        load_layouts(base, prefix, &layouts, engine, computed);
    }
}

fn load_partials(
    base: &PathBuf,
    prefix: &PathBuf,
    dir: &Path,
    engine: &TemplateEngine,
    computed: &mut Plugin,
) {
    let ext = OsStr::new(engine.get_raw_extension());
    let files = walk::find(dir, |e| {
        if let Some(extension) = e.extension() {
            return extension == ext;
        }
        false
    });

    if !files.is_empty() {
        let engine_templates = computed
            .templates_mut()
            .entry(engine.clone())
            .or_insert(Default::default());
        let partials =
            engine_templates.partials.get_or_insert(Default::default());
        files.iter().filter(|e| e.is_file()).for_each(|e| {
            let mut tpl = TemplateAsset {
                file: UrlPath::from(prefix.join(e.strip_prefix(&base).unwrap())),
                schema: None,
            };
            let key = e.file_stem().unwrap().to_string_lossy().into_owned();

            let mut s = e.to_path_buf();
            s.set_extension(config::JSON);
            if s.exists() && s.is_file() {
                tpl.schema =
                    Some(UrlPath::from(prefix.join(s.strip_prefix(&base).unwrap())));
            }

            partials.entry(key).or_insert(tpl);
        });
    }
}

fn load_layouts(
    base: &PathBuf,
    prefix: &PathBuf,
    dir: &Path,
    engine: &TemplateEngine,
    computed: &mut Plugin,
) {

    let ext = OsStr::new(engine.get_raw_extension());
    let files = walk::find(dir, |e| {
        if let Some(extension) = e.extension() {
            return extension == ext;
        }
        false
    });

    if !files.is_empty() {
        let engine_templates = computed
            .templates_mut()
            .entry(engine.clone())
            .or_insert(Default::default());
        let layouts =
            engine_templates.layouts.get_or_insert(Default::default());
        files.iter().filter(|e| e.is_file()).for_each(|e| {
            let tpl = TemplateAsset {
                file: UrlPath::from(prefix.join(e.strip_prefix(&base).unwrap())),
                schema: None,
            };
            let key = e.file_stem().unwrap().to_string_lossy().into_owned();
            layouts.entry(key).or_insert(tpl);
        });
    }
}
