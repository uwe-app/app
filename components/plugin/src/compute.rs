use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

use config::{
    Plugin,
    href::UrlPath,
    style::StyleAsset,
    script::ScriptAsset,
    plugin::TemplateAsset,
    engine::{TemplateEngine, ENGINES},
};

use crate::{Result, walk};

/// Compute plugin information by convention from the file system.
pub(crate) async fn transform(original: &Plugin) -> Result<Plugin> {
    let mut computed = original.clone();
    let base = computed.base().clone();
    let assets = base.join(config::ASSETS);
    let fonts = base.join(config::FONTS);
    let styles = base.join(config::STYLES);
    let scripts = base.join(config::SCRIPTS);

    if assets.exists() && assets.is_dir() {
        load_assets(&base, &assets, &mut computed);
    }

    // Fonts just get placed in the assets collection, this 
    // convention is for convenience so plugin authors can 
    // be more explicit using the file system layout.
    if fonts.exists() && fonts.is_dir() {
        load_assets(&base, &fonts, &mut computed);
    }

    if styles.exists() && styles.is_dir() {
        load_styles(&base, &styles, &mut computed);
    }

    if scripts.exists() && scripts.is_dir() {
        load_scripts(&base, &scripts, &mut computed);
    }

    for engine in ENGINES.iter() {
        let dir = base.join(engine.to_string());
        if dir.exists() && dir.is_dir() {
            load_engine(&base, &dir, &mut computed, &engine);
        }
    }

    println!("Computed data {:#?}", &computed);
    println!("Computed data {}", toml::to_string(&computed)?);

    Ok(computed)
}

fn load_assets(base: &PathBuf, dir: &Path, computed: &mut Plugin) {
    let files = walk::find(dir, |_| {true});
    if !files.is_empty() {
        let items = files
            .iter()
            .filter(|e| e.is_file())
            .map(|e| {
                UrlPath::from(e.strip_prefix(&base).unwrap()) 
            })
            .collect::<HashSet<_>>();
        let existing = computed
            .assets
            .clone()
            .unwrap_or(Default::default());

        let assets: HashSet<_> = items.union(&existing).cloned().collect();
        computed.assets = Some(assets);
    }
}

fn load_styles(base: &PathBuf, dir: &Path, computed: &mut Plugin) {
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
                StyleAsset::from(
                    UrlPath::from(e.strip_prefix(&base).unwrap()))
            })
            .collect::<Vec<_>>();

        let mut existing = computed.styles
            .clone()
            .unwrap_or(Default::default());

        items.append(&mut existing);

        // Ensure we don't have any duplicates
        let mut uniques = HashSet::new();
        items.retain(|e| uniques.insert(e.clone()));

        computed.styles = Some(items);
    }
}

fn load_scripts(base: &PathBuf, dir: &Path, computed: &mut Plugin) {
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
                ScriptAsset::from(
                    UrlPath::from(e.strip_prefix(&base).unwrap()))
            })
            .collect::<Vec<_>>();

        let mut existing = computed.scripts
            .clone()
            .unwrap_or(Default::default());

        items.append(&mut existing);

        // Ensure we don't have any duplicates
        let mut uniques = HashSet::new();
        items.retain(|e| uniques.insert(e.clone()));

        computed.scripts = Some(items);
    }
}

fn load_engine(
    base: &PathBuf,
    dir: &Path,
    computed: &mut Plugin,
    engine: &TemplateEngine) {

    let partials = dir.join(config::PARTIALS);
    let layouts = dir.join(config::LAYOUTS);

    if partials.exists() && partials.is_dir() {
        load_partials(base, &partials, computed, engine);
    }
    if layouts.exists() && layouts.is_dir() {
        load_layouts(base, &layouts, computed, engine);
    }
}

fn load_partials(
    base: &PathBuf,
    dir: &Path,
    computed: &mut Plugin,
    engine: &TemplateEngine) {

    let ext = OsStr::new(engine.get_raw_extension());
    let files = walk::find(dir, |e| {
        if let Some(extension) = e.extension() {
            return extension == ext;
        }
        false 
    });

    if !files.is_empty() {
        //let master_templates = computed.templates;
            //.get_or_insert(Default::default());
        let engine_templates =
            computed.templates
            .entry(engine.clone())
            .or_insert(Default::default());
        let partials = engine_templates.partials
            .get_or_insert(Default::default());
        files
            .iter()
            .filter(|e| e.is_file())
            .for_each(|e| {
                let mut tpl = TemplateAsset{
                    file: UrlPath::from(e.strip_prefix(&base).unwrap()),
                    schema: None,
                };
                let key = e.file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();

                let mut s = e.to_path_buf();
                s.set_extension(config::JSON);
                if s.exists() && s.is_file() {
                    tpl.schema = Some(
                        UrlPath::from(s.strip_prefix(&base).unwrap())
                    ); 
                }

                partials.entry(key).or_insert(tpl);
            });
    }
}

fn load_layouts(
    base: &PathBuf,
    dir: &Path,
    computed: &mut Plugin,
    engine: &TemplateEngine) {

    let ext = OsStr::new(engine.get_raw_extension());
    let files = walk::find(dir, |e| {
        if let Some(extension) = e.extension() {
            return extension == ext;
        }
        false 
    });

    if !files.is_empty() {
        //let master_templates = computed.templates;
            //.get_or_insert(Default::default());
        let engine_templates =
            computed.templates
            .entry(engine.clone())
            .or_insert(Default::default());
        let layouts = engine_templates.layouts
            .get_or_insert(Default::default());
        files
            .iter()
            .filter(|e| e.is_file())
            .for_each(|e| {
                let tpl = TemplateAsset{
                    file: UrlPath::from(e.strip_prefix(&base).unwrap()),
                    schema: None,
                };
                let key = e.file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                layouts.entry(key).or_insert(tpl);
            });
    }
}
