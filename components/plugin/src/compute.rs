use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

use config::{Plugin, href::UrlPath, style::StyleAsset};

use crate::{Result, walk};

/// Compute plugin information by convention from the file system.
pub(crate) async fn transform(original: &Plugin) -> Result<Plugin> {
    let mut computed = original.clone();
    let base = computed.base().clone();
    let assets = base.join(config::ASSETS);
    let fonts = base.join(config::FONTS);
    let scripts = base.join(config::SCRIPTS);
    let styles = base.join(config::STYLES);

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

    println!("Computed data {:?}", computed);
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
            .collect::<HashSet<UrlPath>>();
        let existing = computed.assets.clone().unwrap_or(Default::default());
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
            .collect::<Vec<StyleAsset>>();

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

