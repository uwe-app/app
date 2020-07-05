use std::path::Path;
use std::path::PathBuf;

use thiserror::Error;

use config::{Config, ExtensionConfig};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
}

#[derive(Debug)]
pub enum FileType {
    Markdown,
    Template,
    Unknown,
}

static INDEX_STEM: &str = "index";
static HTML: &str = "html";

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

pub fn get_filters(source: &PathBuf, config: &Config) -> Vec<PathBuf> {
    let mut filters: Vec<PathBuf> = Vec::new();

    let config_file = config.file.clone();

    let partials = config
        .get_partials_path(source);
    let includes = config
        .get_includes_path(source);
    let generator = config
        .get_datasources_path(source);
    let resource = config
        .get_resources_path(source);
    let theme = config
        .get_book_theme_path(source);

    filters.push(partials);
    filters.push(includes);
    filters.push(generator);
    filters.push(resource);

    if let Some(config_file) = &config_file {
        filters.push(config_file.clone());
    }

    if let Some(ref book) = config.book {
        let mut paths = book.get_paths(source);
        filters.append(&mut paths);
    }

    if let Some(ref theme) = theme {
        filters.push(theme.clone());
    }

    if let Some(locales_dir) = config.get_locales(source) {
        filters.push(locales_dir);
    }

    if let Some(ref hooks) = config.hook {
        for (_, v) in hooks {
            if let Some(source) = &v.source {
                let mut buf = source.clone();
                buf.push(source);
                filters.push(buf);
            }
        }
    }

    // NOTE: layout comes from the build arguments so callers
    // NOTE: need to add this to the list of filters if necessary

    filters
}
