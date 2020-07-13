use std::path::PathBuf;

use crate::{INDEX_STEM};
use config::{Config, RuntimeOptions, RenderTypes};

// Try to find a source file for the given URL
pub fn lookup_in(
    _config: &Config,
    options: &RuntimeOptions,
    base: &PathBuf,
    href: &str,
    types: &RenderTypes,
) -> Option<PathBuf> {

    let rewrite_index = options.settings.should_rewrite_index();

    let mut url = href.to_string().clone();
    url = utils::url::trim_slash(&url).to_owned();

    let is_dir = utils::url::is_dir(&url);

    let mut buf = base.clone();
    buf.push(&utils::url::to_path_separator(&url));

    // Check if the file exists directly
    if buf.exists() {
        return Some(buf);
    }

    // Check index pages
    if is_dir {
        let mut idx = base.clone();
        idx.push(&utils::url::to_path_separator(&url));
        idx.push(INDEX_STEM);
        for ext in types.render() {
            idx.set_extension(ext);
            if idx.exists() {
                return Some(buf);
            }
        }
    }

    // Check for lower-level files that could map
    // to index pages
    if rewrite_index && is_dir {
        for ext in types.render() {
            buf.set_extension(ext);
            if buf.exists() {
                return Some(buf);
            }
        }
    }

    None
}

fn lookup_allow(config: &Config, base: &PathBuf, href: &str) -> Option<PathBuf> {
    if let Some(ref link) = config.link {
        if let Some(ref allow) = link.allow {
            for link in allow {
                let url = link.trim_start_matches("/");
                if url == href {
                    let mut buf = base.clone();
                    buf.push(url);
                    return Some(buf);
                }
            }
        }
    }
    None
}

// Try to find a source file for the given URL
pub fn lookup(config: &Config, options: &RuntimeOptions, href: &str) -> Option<PathBuf> {

    let base = &options.source;
    let types = options.settings.types.as_ref().unwrap();

    // Try to find a direct corresponding source file
    if let Some(source) = lookup_in(config, options, base, href, types) {
        return Some(source);
    }

    // Try to find a resource
    let resource = options.get_resources_path();
    if let Some(resource) = lookup_in(config, options, &resource, href, types) {
        return Some(resource);
    }

    // Explicit allow list in site.toml
    if let Some(source) = lookup_allow(config, base, href) {
        return Some(source);
    }

    None
}

pub fn exists( config: &Config, options: &RuntimeOptions, href: &str) -> bool {
    lookup(config, options, href).is_some()
}

