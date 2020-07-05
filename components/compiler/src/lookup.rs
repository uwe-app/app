use std::path::PathBuf;

use config::ExtensionConfig;

use crate::{INDEX_STEM};

use super::context::Context;

// Try to find a source file for the given URL
pub fn lookup_in(
    base: &PathBuf,
    context: &Context,
    href: &str,
    extensions: &ExtensionConfig,
) -> Option<PathBuf> {

    let rewrite_index = context.options.rewrite_index;

    let mut url = href.to_string().clone();
    url = utils::url::trim_slash(&url).to_owned();

    let is_dir = utils::url::is_dir(&url);

    let mut buf = base.clone();
    buf.push(&utils::url::to_path_separator(&url));

    // Check if the file exists directly
    if buf.exists() {
        return Some(buf);
    }

    // FIXME: use ExtensionConfig

    // Check index pages
    if is_dir {
        let mut idx = base.clone();
        idx.push(&utils::url::to_path_separator(&url));
        idx.push(INDEX_STEM);
        for ext in extensions.render.iter() {
            idx.set_extension(ext);
            if idx.exists() {
                return Some(buf);
            }
        }
    }

    // Check for lower-level files that could map
    // to index pages
    if rewrite_index && is_dir {
        for ext in extensions.render.iter() {
            buf.set_extension(ext);
            if buf.exists() {
                return Some(buf);
            }
        }
    }

    None
}

fn lookup_allow(base: &PathBuf, context: &Context, href: &str) -> Option<PathBuf> {
    if let Some(ref link) = context.config.link {
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
pub fn lookup(context: &Context, href: &str) -> Option<PathBuf> {
    let base = &context.options.source;

    let extensions = context.config.extension.as_ref().unwrap();

    // Try to find a direct corresponding source file
    if let Some(source) = lookup_in(base, context, href, extensions) {
        return Some(source);
    }

    // Try to find a resource
    let resource = context.config.get_resources_path(base);
    if let Some(resource) = lookup_in(&resource, context, href, extensions) {
        return Some(resource);
    }

    // Explicit allow list in site.toml
    if let Some(source) = lookup_allow(base, context, href) {
        return Some(source);
    }

    None
}

pub fn source_exists(context: &Context, href: &str) -> bool {
    //lookup(&base, href, clean_url).is_some() || lookup_generator(href, clean_url).is_some()
    lookup(context, href).is_some()
}

