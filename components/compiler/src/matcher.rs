use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
pub enum FileType {
    Markdown,
    Template,
    Unknown,
}

use config::ExtensionConfig;
use utils;

use crate::{Error, HTML, INDEX_STEM};

use super::context::Context;

fn resolve_dir_index<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> Option<PathBuf> {
    let mut buf = file.as_ref().to_path_buf();
    buf.push(INDEX_STEM);
    for ext in extensions.render.iter() {
        buf.set_extension(ext);
        if buf.exists() {
            return Some(buf);
        }
    }
    None
}

pub fn resolve_parent_index<P: AsRef<Path>>(
    file: P,
    extensions: &ExtensionConfig,
) -> Option<PathBuf> {
    if let Some(parent) = file.as_ref().parent() {
        // Not an index file so a single level is sufficient
        if !is_index(&file) {
            return resolve_dir_index(&parent, extensions);
        // Otherwise go back down one more level
        } else {
            if let Some(parent) = parent.parent() {
                return resolve_dir_index(&parent, extensions);
            }
        }
    }
    None
}

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

pub fn lookup_allow(base: &PathBuf, context: &Context, href: &str) -> Option<PathBuf> {
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

pub fn is_index<P: AsRef<Path>>(file: P) -> bool {
    if let Some(nm) = file.as_ref().file_stem() {
        if nm == INDEX_STEM {
            return true;
        }
    }
    false
}

pub fn get_type<P: AsRef<Path>>(p: P, extensions: &ExtensionConfig) -> FileType {
    let file = p.as_ref();
    if let Some(ext) = file.extension() {
        let ext = ext.to_string_lossy().into_owned();
        if extensions.render.contains(&ext) {
            if extensions.markdown.contains(&ext) {
                return FileType::Markdown;
            } else {
                return FileType::Template;
            }
        }
    }
    FileType::Unknown
}

fn has_parse_file_match<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> bool {
    let path = file.as_ref();
    let mut copy = path.to_path_buf();
    for ext in extensions.render.iter() {
        copy.set_extension(ext);
        if copy.exists() {
            return true;
        }
    }
    false
}

pub fn is_clean<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> bool {
    let target = file.as_ref().to_path_buf();
    let result = target.clone();
    return rewrite_index_file(target, result, extensions).is_some();
}

fn rewrite_index_file<P: AsRef<Path>>(file: P, result: P, extensions: &ExtensionConfig) -> Option<PathBuf> {
    let clean_target = file.as_ref();
    if !is_index(&clean_target) {
        if let Some(parent) = clean_target.parent() {
            if let Some(stem) = clean_target.file_stem() {
                let mut target = parent.to_path_buf();
                target.push(stem);
                target.push(INDEX_STEM);

                if !has_parse_file_match(&target, extensions) {
                    let clean_result = result.as_ref().clone();
                    if let Some(parent) = clean_result.parent() {
                        if let Some(stem) = clean_result.file_stem() {
                            let mut res = parent.to_path_buf();
                            res.push(stem);
                            res.push(INDEX_STEM);
                            res.set_extension(HTML);
                            return Some(res);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn relative_to<P: AsRef<Path>>(file: P, base: P, target: P) -> Result<PathBuf, Error> {
    let f = file.as_ref().canonicalize()?;
    let b = base.as_ref().canonicalize()?;
    let mut t = target.as_ref().to_path_buf();
    let relative = f.strip_prefix(b)?;
    t.push(relative);
    Ok(t)
}

// Build the direct destination file path.
pub fn direct_destination<P: AsRef<Path>>(
    source: P,
    target: P,
    file: P,
    base_href: &Option<String>,
) -> Result<PathBuf, Error> {
    let pth = file.as_ref();

    // NOTE: When watching files we can get absolute
    // NOTE: paths passed for `file` even when `source`
    // NOTE: is relative. This handles that case by making
    // NOTE: the `source` absolute based on the current working
    // NOTE: directory.
    let mut src: PathBuf = source.as_ref().to_path_buf();
    if pth.is_absolute() && src.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            src = cwd.clone();
            src.push(source.as_ref())
        }
    }

    let mut relative = pth.strip_prefix(src)?;

    if let Some(ref base) = base_href {
        if relative.starts_with(base) {
            relative = relative.strip_prefix(base)?;
        }
    }

    let result = target.as_ref().clone().join(relative);
    return Ok(result);
}

// Build the destination file path and update the file extension.
pub fn destination<P: AsRef<Path>>(
    source: P,
    target: P,
    file: P,
    file_type: &FileType,
    extensions: &ExtensionConfig,
    rewrite_index: bool,
    base_href: &Option<String>,
) -> Result<PathBuf, Error> {
    let pth = file.as_ref().to_path_buf().clone();
    let result = direct_destination(source, target, file, base_href);
    match result {
        Ok(mut result) => {
            match file_type {
                FileType::Markdown | FileType::Template => {
                    if let Some(ext) = pth.extension() {
                        let ext = ext.to_string_lossy().into_owned();
                        for (k, v) in &extensions.map {
                            if ext == *k {
                                result.set_extension(v);
                                break;
                            }
                        }
                    }

                    if rewrite_index {
                        if let Some(res) = rewrite_index_file(pth.as_path(), result.as_path(), extensions) {
                            result = res;
                        }
                    }
                }
                _ => {}
            }
            return Ok(result);
        }
        Err(e) => return Err(e),
    }
}
